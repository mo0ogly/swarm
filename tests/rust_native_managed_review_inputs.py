#!/usr/bin/env python3
"""Inspect complete actual managed candidate inputs without native review authority.

Real isolated workers, immutable handoff, candidate and three actual controls
precede source inspection. Saved caps, raw-source/report refusals and restoration
use public operations. A committed link is quoted literally, never traversed;
Root/untracked private sources stay absent. Optional old-binary refusal records a
reusable actual fixture. No SQL writes, paid reviewer, acceptance or watchdog.
"""
import argparse,json,os,traceback,uuid
from pathlib import Path
import rust_native_managed_validation as native
import rust_native_review_inputs as ordinary
from rust_native_task_validation import invoke


def run(binary,profile,out,*,reuse=None,before=False,binary_source=False,context_bytes=None,pending_dependent=False):
    """Run one complete readonly inspection or an explicitly unsupported source.

    Existing setup reuse retains actual Root/control originals, not copied state.
    An explicit context_bytes fixture argument is persisted through public quotas;
    None keeps its actual saved value, zero means no additional prompt cap.
    pending_dependent preplans one dependency-blocked worker for the subsequent
    publication/copy journey; it is excluded from the control/review cohort.
    Every injected refusal follows a valid applicable witness and restores exact
    bytes/settings. A successful read grants no provider call or verdict.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);fixture=native.fixture
    qualification=json.loads(profile.read_bytes());assert qualification['no_fixture_feature'] and qualification['sha256']==fixture.digest(binary.read_bytes())
    if reuse:
        setup=json.loads((reuse/'setup.json').read_bytes());root=Path(setup['root']);work=setup['work'];workspace=Path(setup['workspace']);checkpoint=setup['candidate'];binary_source=setup['binary_source']
    else:
        entries={'whole-source.txt':b'Complete extra tracked source for candidate inspection.\n','literal-link':('symlink','../../do-not-read-link-target')}
        if binary_source:entries['binary-input.bin']=b'\xff\xfe\x00Not a text review input'
        native.managed.worker.subreaper_scope(lambda:native.managed.run(binary,profile,out/'workers',project_subdir='app',controls=native.CONTROLS,source_entries=entries,pending_dependent=pending_dependent),lambda error:None,native.managed.parent_death.subreaper)
        birth=json.loads((out/'workers/fixture.json').read_bytes());root=Path(birth['root']);work=birth['work']
    environment=dict(os.environ);environment['CODEX_HOME']=str(root/'codex-home');checks=[]
    # A reused actual Root retains old quota event IDs; each new journey needs
    # a fresh fixture namespace, never a replay with changed revision/intent.
    namespace=uuid.uuid4().hex
    def set_quotas(label,**patch):
        """Persist a fresh fixture quota intent without conflicting with history."""
        return ordinary.quotas(binary,root,out,label+'-'+namespace,work,**patch)
    def call(label,args,value=None):return invoke(binary,root,out,label,args,value,environment=environment)
    def ok(label,args,value=None):
        code,value=call(label,args,value);assert code==0,(label,value);return value
    def show(label):return ok(label,['work','show',work])['work']
    def request(label):return dict(schema_version=1,event_id=label+'-'+out.name,expected_revision=show(label+'-work')['revision'],task_id='task')
    def inspect(label,*,expected=True,action='review-inputs'):
        """Read the public inspector and prove no Work/call or process mutation."""
        state=show(label+'-before');markers={p.name:fixture.digest(p.read_bytes()) for p in root.glob('model-received*')}
        code,reply=call(label,['task',action,work],dict(schema_version=1,event_id=label,expected_revision=state['revision'],task_id='task'))
        assert (code==0)==expected,(label,reply)
        assert show(label+'-after')==state and {p.name:fixture.digest(p.read_bytes()) for p in root.glob('model-received*')}==markers
        if expected:
            preview=reply['preview'] if action=='review-inputs' else reply
            assert not preview['invocation_authorized'] and not preview['provider_started'] and not preview['task_accepted'] and preview['native_control_evidence_verified']
        return reply
    if not reuse:
        state=show('mission-work');mission=ok('mission',['mission','show',work])
        if mission['paused']:ok('resume-mission',['mission','resume',work],dict(schema_version=1,event_id='inspect-resume-mission',expected_revision=state['revision'],reason='Explicit isolated candidate inspection'))
        state=show('planning-work')
        if state['planning'].get('paused'):ok('resume-planning',['planning','resume',work],dict(schema_version=1,event_id='inspect-resume-planning',expected_revision=state['revision'],reason='Explicit isolated candidate inspection'))
        value=ok('candidate',['task','validation-candidate',work],request('candidate'));assert value['control_count']==3
        current=show('repository');storage=Path(current['planning']['repository']['storage']);found=[json.loads(f.read_bytes()) for f in (storage/'candidates').glob('*/candidate.json') if json.loads(f.read_bytes())['task']=='task'];assert len(found)==1;checkpoint=found[0];workspace=Path(checkpoint['workspace'])
        controls=ok('controls',['task','validate',work],request('controls'));assert controls['controls_passed'] and not controls['task_accepted']
        setup=dict(root=str(root),work=work,workspace=str(workspace),candidate=checkpoint,binary_source=binary_source)
    fixture.publish(out/'setup.json',setup)
    (root/'do-not-read-link-target').write_text('UNSHARED-ROOT-PRIVATE-LINK-CONTENT')
    (workspace/'untracked-review-private.txt').write_text('UNTRACKED-PRIVATE-SOURCE-NOT-IN-CANDIDATE')
    if before:
        refused=inspect('prior-managed-input-refusal',expected=False)
        assert 'review-managed-candidate-unported' in json.dumps(refused),refused
        fixture.publish(out/'result.json',dict(status='BASELINE_REFUSAL',actual_controls_passed=True,root=str(root),work=work,task_accepted=False));print('MANAGED_REVIEW_BASELINE_REFUSAL');return
    if context_bytes is not None:set_quotas('explicit-fixture-prompt-cap',review_context_bytes=context_bytes)
    inspect('valid-preview',action='review-preview')
    if binary_source:
        refused=inspect('nontext-refusal',expected=False);assert 'source-utf8-required' in json.dumps(refused),refused
        checks=['actual-controls-before-nontext-inspection','valid-readonly-preview-before-nontext-refusal','nontext-candidate-refuses-without-provider-or-work-change']
    else:
        inputs=inspect('complete');body=json.loads(inputs['prompt'].rsplit('Do not use tools.\n',1)[1]);sources={s['name']:s for s in inputs['materials']}
        assert inputs['prompt_sha256']==fixture.digest(inputs['prompt'].encode()) and inputs['source_bytes']==sum(len(s['content'].encode()) for s in inputs['materials'])
        assert all(s['sha256']==fixture.digest(s['content'].encode()) for s in inputs['materials'])
        assert body['inspection_only'] and not body['native_invocation_supported'] and not body['native_verdict_supported'] and not body['accepted_baseline_verdicts_verified']
        assert body['candidate']['candidate_commit']==checkpoint['commit'] and body['candidate']['candidate_binding_sha256']==checkpoint['binding_sha256']
        assert len(body['candidate']['control_observations'])==3 and len(body['tasks'])==1 and body['tasks'][0]['task_id']=='task'
        inventory={v['path']:v for v in body['tracked_sources']};assert set(inventory)==set(checkpoint['tracked'])
        for name,entry in checkpoint['tracked'].items():
            key=('candidate-link/' if entry['mode']=='120000' else 'candidate/')+name
            assert sources[key]['sha256']==entry['sha256'] and inventory[name]['sha256']==entry['sha256']
        assert sources['candidate/app/whole-source.txt']['content']=='Complete extra tracked source for candidate inspection.\n'
        assert sources['candidate-link/app/literal-link']['content']=='../../do-not-read-link-target'
        assert 'UNSHARED-ROOT-PRIVATE-LINK-CONTENT' not in inputs['prompt'] and 'UNTRACKED-PRIVATE-SOURCE-NOT-IN-CANDIDATE' not in inputs['prompt'] and 'PRIVATE dirty source excluded' not in inputs['prompt']
        assert not any(n.startswith('cohort/task-2/') for n in sources)
        checks+=['complete-tracked-candidate-and-full-control-cohort','literal-symlink-target-without-traversal','no-root-or-untracked-source-scan','read-only-source-and-control-context']
        for field,reason in [('review_context_files','review-context-files-exceeded'),('review_source_bytes','source-exceeds-configured-cap'),('review_context_bytes','review-context-bytes-exceeded'),('review_delivery_files','review-delivery-files-exceeded')]:
            inspect(field+'-witness');old=set_quotas(field+'-save',**{field:1})
            refused=inspect(field+'-refused',expected=False);assert reason in json.dumps(refused),(field,refused)
            set_quotas(field+'-restore',**{field:old[field]})
            restored=inspect(field+'-restored');assert restored['prompt_sha256']==inputs['prompt_sha256']
            checks.append(field+'-persisted-refusal-and-restoration')
        saved=(workspace/'tracked.txt').read_bytes();(workspace/'tracked.txt').write_bytes(b'Changed tracked bytes\n')
        try:
            refused=inspect('changed-source',expected=False);assert 'managed-candidate-raw-source-changed' in json.dumps(refused),refused
        finally:(workspace/'tracked.txt').write_bytes(saved)
        assert inspect('source-restored')['prompt_sha256']==inputs['prompt_sha256'];checks.append('changed-raw-source-refuses-and-restores')
        task=next(t for t in show('report-work')['tasks'] if t['id']=='task');report=root/body['tasks'][0]['report_path'];parked=report.with_suffix('.inspection-retained');report.rename(parked)
        try:inspect('missing-report',expected=False)
        finally:parked.rename(report)
        assert inspect('report-restored')['prompt_sha256']==inputs['prompt_sha256'];checks.append('missing-report-refuses-and-restores')
        zero_fields=['review_context_files','review_source_bytes','review_context_bytes','review_delivery_files'];old=set_quotas('zero-caps',**dict.fromkeys(zero_fields,0));assert inspect('zero-caps-read')['prompt_sha256']==inputs['prompt_sha256'];set_quotas('restore-caps',**{k:old[k] for k in zero_fields});inspect('caps-restored');checks.append('saved-zero-caps-mean-no-additional-limit')
    state=show('before-native-guard');code,wire=call('actual-review-guard',['task','review',work],dict(schema_version=1,event_id='native-review-stale-revision',expected_revision=state['revision']+1,task_id='task'));assert code!=0 and 'review-revision-conflict' in json.dumps(wire),wire
    after=show('after-native-guard');assert after==state and after['planning']['reviewer'].get('calls',0)==0 and not any(t.get('independent_review') for t in after['tasks'])
    checks.append('actual-native-review-requires-current-revision-without-call-or-acceptance')
    result=dict(status='PASS',root=str(root),work=work,binary_source=binary_source,checks=checks,reviewer_started=False,accepted=False,global_watchdog=None)
    fixture.publish(out/'result.json',result);print('MANAGED_REVIEW_INPUTS_PASS '+str(len(checks))+' assertions')


def main():
    """Retain exact helper/normal-profile inputs and original failure channels."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--review-context-bytes',type=int);parser.add_argument('--reuse-from',type=Path);parser.add_argument('--before',action='store_true');parser.add_argument('--binary-source',action='store_true');a=parser.parse_args();binary=a.binary.resolve();profile=a.profile.resolve();out=a.output.resolve()
    inputs=dict(binary=str(binary),binary_sha256=native.fixture.digest(binary.read_bytes()),profile_sha256=native.fixture.digest(profile.read_bytes()),helpers=native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(binary,profile,out,reuse=a.reuse_from.resolve() if a.reuse_from else None,before=a.before,binary_source=a.binary_source,context_bytes=a.review_context_bytes)
        assert native.fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and native.fixture.digest(profile.read_bytes())==inputs['profile_sha256'] and all(native.fixture.digest(Path(n).read_bytes())==h for n,h in inputs['helpers'].items())
        native.fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.is_dir():native.fixture.publish(out/'inputs.json',inputs);native.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
