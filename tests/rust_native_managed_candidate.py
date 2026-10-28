#!/usr/bin/env python3
"""Exercise actual Root candidate preparation after real managed worker handoffs.

Uses isolated native planning/worker fixtures and public CLI only. No SQL writes,
paid model, native validation command, independent review or task acceptance.
Public readback and read-only SQL prove unchanged Work/jobs. A PATH Git wrapper
performs one public settings mutation while Git is running: successful mutation
proves Root/settings locks were released; final observation must refuse stale
preparation. All original channels, candidate files and failure evidence survive.
"""
import argparse, ast, hashlib, json, os, shutil, sqlite3, subprocess, sys, traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_managed_auto_worker as managed
from rust_native_task_validation import invoke


def helper_inputs(entry):
    """Hash the static local Python import closure without executing discovery.

    Starts at this helper, resolves only sibling .py files and follows absolute
    import/from module names. Standard libraries and absent modules are ignored;
    this does not prove dynamic imports, compiler inputs or independent acceptance.
    Retained recipe/profile inventories separately bind those declared contracts.
    """
    pending=[entry.resolve()];seen={}
    while pending:
        path=pending.pop()
        if str(path) in seen:continue
        raw=path.read_bytes();seen[str(path)]=fixture.digest(raw)
        for node in ast.walk(ast.parse(raw)):
            names=[a.name for a in node.names] if isinstance(node,ast.Import) else [node.module] if isinstance(node,ast.ImportFrom) and node.module else []
            for name in names:
                candidate=entry.parent/(name.split('.')[0]+'.py')
                if candidate.is_file():pending.append(candidate.resolve())
    return seen


def job_rows(root):
    """Read actual validation rows without creating a missing table or database."""
    db=sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True)
    try:
        tables={row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        return {name:db.execute('SELECT * FROM '+name+' ORDER BY rowid').fetchall()
            for name in ['native_validation_jobs','native_validation_steps','native_validation_source_origins'] if name in tables}
    finally: db.close()


def run(binary,profile,out,*,before_binary=None,subproject=False,workers_evidence=None):
    """Verify a valid current native producer before faults and recovery.

    Optional workers_evidence resumes an actual PASS setup bound to this binary;
    it never copies, edits or invents producer state. Existing managed setup runs
    actual workers, retains result commits and submits
    through the owner inbox. Only fixture missions are resumed by public controls;
    no service remains active during preparation. Private candidate corruption is
    isolated test injection with exact restore, never an operator recovery API.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    workers=workers_evidence or out/'workers'
    if workers_evidence is None:
        managed.worker.subreaper_scope(
            lambda: managed.run(binary,profile,workers,project_subdir='app' if subproject else '',controls=[dict(id='candidate-input',command=['python3','-c',"from pathlib import Path; assert Path('tracked.txt').read_text().strip()"],criteria=[1],inputs=['tracked.txt'],justification='Check actual candidate input without executing during preparation')]),
            lambda error: None, managed.parent_death.subreaper)
    recorded=json.loads((workers/'result.json').read_bytes())
    assert recorded['status']=='PASS' and recorded['binary_sha256']==fixture.digest(binary.read_bytes())
    birth=json.loads((workers/'fixture.json').read_bytes());root=Path(birth['root']);work=birth['work']
    fixture_environment=dict(os.environ);fixture_environment['CODEX_HOME']=str(root/'codex-home')
    checks=[]
    def call(label,args,value=None,env=None,exe=None):
        return invoke(exe or binary,root,out,label,args,value,environment=env if env is not None else fixture_environment)
    def ok(label,args,value=None):
        code,value=call(label,args,value);assert code==0,(label,value);return value
    def show(label):return ok(label,['work','show',work])['work']
    def passed(label):checks.append(label);print('PASS '+label,flush=True)
    current=show('initial')
    mission=ok('mission-before',['mission','show',work])
    if mission['paused']:
        ok('resume-mission',['mission','resume',work],dict(schema_version=1,event_id='candidate-mission-resume-'+out.name,expected_revision=current['revision'],reason='Owned candidate preparation without running conductor'))
    current=show('before-planning-resume')
    if current['planning'].get('paused',False):
        ok('resume-planning',['planning','resume',work],dict(schema_version=1,event_id='candidate-planning-resume-'+out.name,expected_revision=current['revision'],reason='Owned candidate preparation only'))
    current=show('before');task=next(t for t in current['tasks'] if t['id']=='task')
    assert task['status']=='submitted' and len(task['attempts'])==1
    request=dict(schema_version=1,event_id='candidate-only',expected_revision=current['revision'],task_id='task')
    rows=job_rows(root);storage=Path(current['planning']['repository']['storage'])
    if before_binary:
        code,wire=call('before-missing-native-candidate',['task','validation-candidate',work],request,exe=before_binary)
        assert code!=0 and not (storage/'candidates').exists(),wire
        passed('prior-native-binary-lacks-candidate-operation-without-side-effects')
    result=ok('prepare',['task','validation-candidate',work],request)
    assert result['work_revision']==current['revision'] and result['cohort_task_count']==1 and result['control_count']==1,result
    assert result['producer_agent_id'] in recorded['agent_ids']
    assert len(result['candidate_commit'])==40 and len(result['candidate_binding_sha256'])==64
    assert not result['job_created'] and not result['controls_started'] and not result['task_accepted']
    assert show('unchanged-work')==current and job_rows(root)==rows
    originals=list((storage/'candidates').glob('*/candidate.json'));assert len(originals)==1
    original=originals[0];raw=original.read_bytes();candidate=json.loads(raw)
    assert candidate['commit']==result['candidate_commit'] and len(candidate['control_plan']['policies'])==1
    workspace=Path(candidate['workspace']);assert workspace.is_dir() and workspace.name==('app' if subproject else 'copy')
    stages=sorted(str(v) for v in original.parent.glob('stages/*'))
    passed('actual-native-submission-prepares-retained-candidate-without-job-or-work-mutation')
    def unchanged():
        assert original.read_bytes()==raw and job_rows(root)==rows
        assert sorted(str(v) for v in original.parent.glob('stages/*'))==stages
    assert ok('repeat',['task','validation-candidate',work],request)==result;unchanged()
    passed('unchanged-preparation-reuses-exact-checkpoint-and-stage')
    for label,value,expected in [('stale-revision',{**request,'expected_revision':request['expected_revision']-1},'task-validation-revision-conflict'),('forged-producer',{**request,'agent_id':'fake'},'task-validation-request-invalid')]:
        code,wire=call(label,['task','validation-candidate',work],value)
        assert code!=0 and expected in json.dumps(wire),(label,wire)
        unchanged();assert show(label+'-after')==current
        passed(label+'-refuses-without-mutation')
    tracked=workspace/'tracked.txt';old=tracked.read_bytes();fixture.publish(tracked,b'Owned source fault after valid preparation\n')
    try:
        code,wire=call('changed-source',['task','validation-candidate',work],request)
        assert code!=0 and 'managed-candidate' in json.dumps(wire),wire
        unchanged()
    finally:fixture.publish(tracked,old)
    assert ok('source-recovery',['task','validation-candidate',work],request)==result
    passed('changed-candidate-source-refuses-and-exact-source-restore-recovers')
    # Preparation/preview is still nonmutating; explicit native validate is now
    # implemented separately. Incomplete final managed review remains refused.
    preview=ok('readonly-managed-preview',['task','validation-preview',work],request)
    assert preview['control_count']==result['control_count'] and not preview['execution_authorized']
    code,wire=call('guard-review-preview',['task','review-preview',work],request)
    assert code!=0 and 'review-native-controls-required' in json.dumps(wire),wire
    code,wire=call('guard-actual-review',['task','review',work],request)
    assert code!=0 and 'review-managed-candidate-unported' in json.dumps(wire),wire
    unchanged()
    passed('candidate-preparation-and-preview-do-not-execute-or-unlock-final-review')
    settings=ok('settings-before',['engine-settings','show']);values=settings['values']
    changed=dict(values);changed['agent_supervision_poll_interval_ms']=str(int(values['agent_supervision_poll_interval_ms'])+1)
    # Git wrapper invokes the real public Admin flow once, then execs the actual Git.
    wrapper=out/'git-wrapper';wrapper.mkdir(mode=0o700);marker=out/'settings-during-git.json';real_git=shutil.which('git');assert real_git
    code="""#!/usr/bin/env python3
import json,os,subprocess,sys
from pathlib import Path
sys.path.insert(0,TESTS)
from rust_native_task_validation import invoke
from rust_agent_model_route import publish
if not MARKER.exists():
 d=invoke(BINARY,ROOT,OUT,'during-git-settings-show',['engine-settings','show'])[1]
 r=dict(schema_version=d['schema_version'],event_id='candidate-settings-race',expected_revision=d['revision'],values=VALUES,reason='Owned preparation freshness race',preview_token='')
 c,p=invoke(BINARY,ROOT,OUT,'during-git-preview',['engine-settings','preview'],r);assert c==0,p
 r['preview_token']=p['preview_token'];c,a=invoke(BINARY,ROOT,OUT,'during-git-apply',['engine-settings','apply'],r);assert c==0,a
 publish(MARKER,dict(actual_public_mutation=True,revision=a.get('revision'),no_parent_sql_or_settings_lock=True))
os.execv(REAL_GIT,[REAL_GIT,*sys.argv[1:]])
"""
    constants={'TESTS':str(Path(__file__).resolve().parent),'MARKER':marker,'BINARY':binary,'ROOT':root,'OUT':out,'VALUES':changed,'REAL_GIT':real_git}
    lines=[]
    for name,value in constants.items():lines.append(name+' = '+('Path('+repr(str(value))+')' if isinstance(value,Path) else repr(value)))
    # Define constants before importing helpers from their configured tests path.
    code=code.replace('sys.path.insert(0,TESTS)','\n'.join(lines)+'\nsys.path.insert(0,TESTS)')
    fixture.publish(wrapper/'git',code.encode());(wrapper/'git').chmod(0o700)
    environment=dict(fixture_environment);environment['PATH']=str(wrapper)+os.pathsep+environment['PATH']
    code,wire=call('settings-race',['task','validation-candidate',work],request,env=environment)
    assert code!=0 and 'task-validation-candidate-preparation-changed' in json.dumps(wire),wire
    assert json.loads(marker.read_bytes())['actual_public_mutation'] and show('race-work-unchanged')==current
    unchanged();passed('public-settings-mutation-during-git-succeeds-and-stale-preparation-refuses')
    settings=ok('settings-for-restore',['engine-settings','show'])
    restore=dict(schema_version=settings['schema_version'],event_id='candidate-settings-restore',expected_revision=settings['revision'],values=values,reason='Restore owned preparation fixture values preserving history',preview_token='')
    preview=ok('settings-restore-preview',['engine-settings','preview'],restore);restore['preview_token']=preview['preview_token']
    ok('settings-restore-apply',['engine-settings','apply'],restore)
    assert ok('settings-recovery',['task','validation-candidate',work],request)==result;unchanged()
    passed('public-settings-restore-recovers-same-original-with-history-preserved')
    proof=dict(status='PASS',assertions=checks,root=str(root),work=work,candidate=result,
        actual_native_producer=True,private_sql_writes=False,controls_started=False,accepted=False,
        global_watchdog=None,subproject=subproject,independent_review=False)
    fixture.publish(out/'result.json',proof);return proof


def main():
    """Bind the actual normal binary/profile and helper inputs to retained evidence."""
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    p.add_argument('--before-binary',type=Path);p.add_argument('--workers-evidence',type=Path);p.add_argument('--subproject',action='store_true');a=p.parse_args()
    binary=a.binary.resolve();profile=a.profile.resolve()
    inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),
        profile=str(profile),profile_sha256=fixture.digest(profile.read_bytes()),
        sources=helper_inputs(Path(__file__)),workers_evidence=str(a.workers_evidence.resolve()) if a.workers_evidence else None,
        global_watchdog=None)
    try:
        run(binary,profile,a.output.resolve(),before_binary=a.before_binary.resolve() if a.before_binary else None,
            subproject=a.subproject,workers_evidence=a.workers_evidence.resolve() if a.workers_evidence else None)
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and fixture.digest(profile.read_bytes())==inputs['profile_sha256']
        assert all(fixture.digest(Path(path).read_bytes())==digest for path,digest in inputs['sources'].items())
        fixture.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.is_dir():
            fixture.publish(a.output/'inputs.json',inputs)
            fixture.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
