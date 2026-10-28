#!/usr/bin/env python3
"""Qualify retained native publication/policy/worker stages by current public reads.

Original successful stages may belong to an earlier incomplete composite recipe;
this tool never relabels that recipe PASS or reruns its providers/controls. Exact
stage result/input hashes and actual current originals remain mandatory. No SQL
writes, authority, budget change, new worker or global watchdog is introduced.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_native_cumulative_publication as cumulative
import rust_managed_repository_bootstrap as repository
from rust_native_task_validation import invoke


def run(binary,profile,out,publication,policies,dependent,retained_profile=None):
    """Verify two real publications, applied capacity and completed C2 worker.

    Retains exact references to source stage results/inputs and their declared
    historical helper hashes. These are original executions, not claims those
    helpers were reexecuted. Current canonical baseline, public replay and copy
    content are observed without changing Work/finance/provider invocations.
    retained_profile explicitly permits historical stage binaries from another
    verified normal profile. Current public reads still exercise the supplied
    current binary; no unchanged-build equivalence is inferred. Refusal,
    stale/missing retained data or changed exercised binary fails explicitly.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False);f=cumulative.fixture
    qualified=json.loads(profile.read_bytes());sha=f.digest(binary.read_bytes());assert qualified['sha256']==sha and qualified['no_fixture_feature']
    historical=json.loads(retained_profile.read_bytes()) if retained_profile else qualified
    assert historical['no_fixture_feature'] and historical['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    stages={};refs={}
    if retained_profile:refs['retained_profile']=dict(path=str(retained_profile),sha256=f.digest(retained_profile.read_bytes()),binary_sha256=historical['sha256'])
    for name,path in [('publication',publication),('policies',policies),('dependent',dependent)]:
        result=json.loads((path/'result.json').read_bytes());inputs=json.loads((path/'inputs.json').read_bytes())
        assert result['status']=='PASS' and not result['paid_provider'] and inputs['binary_sha256']==historical['sha256']
        refs[name]=dict(path=str(path),result_sha256=f.digest((path/'result.json').read_bytes()),inputs_sha256=f.digest((path/'inputs.json').read_bytes()),scope='retained actual stage; no reexecution inferred')
        stages[name]=result
    published=stages['publication'];policy=stages['policies'];worker=stages['dependent']
    assert len(published['checks'])==9 and published['cumulative_publication'] and not published['mission_accepted']
    assert len(policy['checks'])==2 and policy['work_unchanged'] and policy['settings_values_restored'] and policy['settings_history_preserved']
    assert len(worker['checks'])==4 and worker['cumulative_publication'] and worker['accepted_task_count']==2 and worker['candidate_commit']==published['candidate_commit']
    capacity=worker['review_capacity_change'];assert capacity['publicly_applied'] and capacity['before']==capacity['consumed'] and capacity['after']==capacity['consumed']+1
    assert all(s['root']==published['root'] and s['work']==published['work'] for s in stages.values())
    f.publish(out/'retained-stages.json',refs)
    root=Path(published['root']);work=published['work'];assert root.is_absolute() and root.name.startswith('swarm-model83-')
    env=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    def call(label,args,value=None):
        """Retain exact actual CLI channels; a refused read is never empty success."""
        code,reply=invoke(binary,root,out,label,args,value,env);assert code==0,(label,reply);return reply
    before=call('before',['work','show',work])['work'];money=cumulative.baseline.workers.publication.native.finance(root,work)
    markers={p.name:f.digest(p.read_bytes()) for p in root.glob('*reviewer-invoked.json')}
    request=dict(schema_version=1,event_id='cumulative-readback',expected_revision=before['revision'],task_id='dependent')
    baseline=call('current-baseline',['task','baseline-preview',work],request)
    assert baseline['baseline_verified'] and len(baseline['tasks'])==2 and baseline['candidate_commit']==published['candidate_commit']
    assert not any(baseline[k] for k in ['publication_authorized','provider_started','task_accepted'])
    checks=['retained-nine-two-four-stage-obligations-bound-to-exact-artifacts','current-two-publication-baseline-still-verifies']
    old=json.loads((publication/'before.json').read_bytes());oldtasks={t['id']:t for t in old['tasks'] if t['status']=='accepted'}
    for id,task in oldtasks.items():assert next(t for t in before['tasks'] if t['id']==id)==task
    for entry in baseline['tasks']:
        input=dict(schema_version=1,event_id=entry['publication_event'],task_id=entry['task_id'])
        # Read original request coordinates from the already verified canonical
        # receipt, never invent a revision or convert a failure into a replay.
        import sqlite3
        connection=sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True)
        with cumulative.restoration(connection.close):
            row=connection.execute('SELECT request FROM events WHERE id=?',(entry['publication_event'],)).fetchone();assert row is not None
            record=json.loads(row[0])
        input['expected_revision']=record['expected_revision']
        replay=call('replay-'+entry['task_id'],['task','publish',work],input)
        assert replay['replayed'] and replay['work']==before and replay['candidate_commit']==record['candidate_commit']
    checks.append('both-original-publication-replays-return-current-work-without-execution')
    agent=call('dependent-agent',['agent','show',worker['agent_id']])['agent'];assert agent['status']=='completed'
    repo=before['planning']['repository'];copy=Path(repo['storage'])/'copies/dependent-1';workspace=Path(agent['workspace'])
    assert repository.git(copy,'rev-parse','HEAD').decode().strip()==published['candidate_commit']
    for id in ['task','task-2']:
        name=(repo.get('project_subdir','')+'/' if repo.get('project_subdir') else '')+'docs/'+id+'.md'
        content=repository.git(Path(repo['storage'])/'repository.git','show',published['candidate_commit']+':'+name)
        assert content and (workspace/('docs/'+id+'.md')).read_bytes()==content
    assert next(t for t in before['tasks'] if t['id']=='dependent')['status']=='blocked'
    checks.append('completed-next-worker-retains-c2-head-and-both-published-reports')
    quota=call('current-quota',['quotas','show',work]);assert quota['limits']['review_calls']==capacity['after'] and quota['consumed']['review_calls']==capacity['consumed']
    assert call('after',['work','show',work])['work']==before and cumulative.baseline.workers.publication.native.finance(root,work)==money and {p.name:f.digest(p.read_bytes()) for p in root.glob('*reviewer-invoked.json')}==markers
    checks.append('public-capacity-consumption-work-and-provider-markers-preserved')
    f.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,retained_stage_counts=[9,2,4],provider_started=False,worker_started=False,mission_accepted=False,binary_sha256=sha))
    print('NATIVE_CUMULATIVE_READBACK_VERIFIED '+str(len(checks)),flush=True)


def main():
    """Retain actual current inputs and all failure diagnostics in fresh evidence."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output','publication-from','policies-from','dependent-from']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--retained-profile',type=Path)
    args=parser.parse_args();out=args.output.resolve();f=cumulative.fixture
    inputs=dict(binary_sha256=f.digest(args.binary.read_bytes()),profile_sha256=f.digest(args.profile.read_bytes()),sources=cumulative.baseline.workers.publication.native.inspection.native.helper_inputs(Path(__file__)))
    try:
        run(args.binary.resolve(),args.profile.resolve(),out,args.publication_from.resolve(),args.policies_from.resolve(),args.dependent_from.resolve(),args.retained_profile.resolve() if args.retained_profile else None)
        assert f.digest(args.binary.read_bytes())==inputs['binary_sha256'] and all(f.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        f.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():f.publish(out/'inputs.json',inputs);f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
