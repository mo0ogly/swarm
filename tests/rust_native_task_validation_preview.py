#!/usr/bin/env python3
"""Verify native task-control preparation through real public CLI operations.

Uses an owned inert planner/worker fixture, no Go seed or private SQL writes.
Preview must not run the declared control, mutate Work or pretend acceptance.
Original commands/responses and exact source/default-binary hashes are retained.
"""
import argparse,json,os,sys,traceback
from pathlib import Path
import rust_native_task_validation as native
fixture=native.fixture
public=native.public

def scenario(binary,out):
    """Check valid repeat, authority/stale refusals, changed/missing artifacts and
    recovery on the same submitted task without creating another worker attempt.

    One separate declared input varies independently of the canonical report;
    its changed digest invalidates the source binding without claiming quality.
    """
    setup=public.flow(binary,out,control_script=native.CONTROL,control_inputs=['handoff.md','validation-input.txt'])
    root=Path(setup['root']);work=setup['work_id'];declared=root/'validation-input.txt';fixture.publish(declared,b'original control input\n')
    r=dict(schema_version=1,event_id='validation-preview',expected_revision=setup['revision'],task_id='task')
    def call(name,request=r):return native.invoke(binary,root,out,name,['task','validation-preview',work],request)
    def unchanged():
        code,shown=native.invoke(binary,root,out,'work-read-'+str(len(list(out.glob('work-read-*.stdout')))),['work','show',work]);assert code==0
        assert shown==before and not (root/'validation-executed.json').exists()
    code,before=native.invoke(binary,root,out,'before-preview',['work','show',work]);assert code==0
    code,valid=call('valid');assert code==0,valid
    assert valid['execution_authorized'] is False and valid['task_accepted'] is False and valid['control_count']==1
    assert valid['producer_agent_id']==setup['agent_id'] and valid['attempt_id']==setup['attempt_id']
    assert valid['work_revision']==setup['revision'] and valid['policy_mode']=='automatic'
    assert valid['artifacts']=={'handoff.md':fixture.digest((root/'handoff.md').read_bytes()),'validation-input.txt':fixture.digest(declared.read_bytes())}
    unchanged();code,repeated=call('repeat');assert code==0 and repeated==valid;unchanged()
    for name,patch in [('actor',{'actor':'operator'}),('origin',{'origin':'automatic'}),('command',{'command':['python3']}),('producer',{'agent_id':'other'}),('stale',{'expected_revision':setup['revision']-1}),('task',{'task_id':'absent'})]:
        code,refused=call(name,dict(r,**patch));assert code!=0,(name,refused);unchanged()
    report=root/'handoff.md';original=report.read_bytes();fixture.publish(report,b'changed report\n')
    code,refused=call('changed-report');assert code!=0;unchanged();fixture.publish(report,original)
    os.rename(report,root/'held-report.md')
    code,refused=call('missing-report');assert code!=0;unchanged();os.rename(root/'held-report.md',report)
    fixture.publish(declared,b'new declared input\n');code,changed=call('changed-input');assert code==0 and changed['source_sha256']!=valid['source_sha256'];assert changed['artifacts']['handoff.md']==valid['artifacts']['handoff.md'];unchanged()
    os.rename(declared,root/'held-input.txt');code,refused=call('missing-input');assert code!=0;unchanged();os.rename(root/'held-input.txt',declared)
    fixture.publish(declared,b'original control input\n');code,restored=call('restored');assert code==0 and restored==valid;unchanged()
    class Owned:
        """Public Admin adapter for this already public owned fixture."""
        def __init__(self):self.root=root;self.counter=0
        def ok(self,args,value=None):
            self.counter+=1;code,reply=native.invoke(binary,root,out,'policy-'+str(self.counter),args,value);assert code==0,(args,reply);return reply
    owned=Owned();settings=owned.ok(['engine-settings','show']);old_delay=settings['values']['storage_busy_retry_delay_ms']
    public.feedback.apply_policy(owned,'change-storage-context',dict(storage_busy_retry_delay_ms='0' if old_delay!='0' else '1'))
    code,unrelated=call('storage-context-changed');assert code==0 and unrelated['source_sha256']==valid['source_sha256'] and unrelated['execution_policy_generation']==valid['execution_policy_generation'];unchanged()
    restored_settings=public.feedback.apply_policy(owned,'restore-storage-context',dict(storage_busy_retry_delay_ms=old_delay));assert restored_settings['values']==settings['values']
    code,restored=call('storage-context-restored');assert code==0 and restored['source_sha256']==valid['source_sha256'] and restored['settings_revision']!=valid['settings_revision'];unchanged()
    poll=settings['values']['agent_supervision_poll_interval_ms'];different=str(int(poll)-1 if int(poll)>1 else 2)
    public.feedback.apply_policy(owned,'change-execution-context',dict(agent_supervision_poll_interval_ms=different))
    code,relevant=call('execution-context-changed');assert code==0 and relevant['source_sha256']!=restored['source_sha256'] and int(relevant['execution_policy_generation'])>int(restored['execution_policy_generation']);unchanged()
    restored_settings=public.feedback.apply_policy(owned,'restore-execution-context',dict(agent_supervision_poll_interval_ms=poll));assert restored_settings['values']==settings['values']
    code,valid=call('execution-context-restored');assert code==0 and valid['source_sha256']!=restored['source_sha256'] and int(valid['execution_policy_generation'])>int(relevant['execution_policy_generation']) and valid['artifacts']==restored['artifacts'];unchanged()
    code,paused=native.invoke(binary,root,out,'pause',['planning','pause',work],dict(schema_version=1,event_id='pause-preview',expected_revision=setup['revision']));assert code==0 and paused['planning']['paused']
    code,before=native.invoke(binary,root,out,'paused-work',['work','show',work]);assert code==0
    code,refused=call('paused',dict(r,expected_revision=paused['revision']));assert code!=0;unchanged()
    code,resumed=native.invoke(binary,root,out,'resume',['planning','resume',work],dict(schema_version=1,event_id='resume-preview',expected_revision=paused['revision']));assert code==0 and not resumed['planning']['paused']
    code,before=native.invoke(binary,root,out,'resumed-work',['work','show',work]);assert code==0
    code,restored=call('resumed',dict(r,expected_revision=resumed['revision']));assert code==0 and restored['source_sha256']!=valid['source_sha256'] and int(restored['scope_generation'])>int(valid['scope_generation']) and restored['authority_event']!=valid['authority_event'] and restored['execution_policy_generation']==valid['execution_policy_generation'];unchanged()
    return dict(status='PASS',root=str(root),groups=9,unrelated_admin_history_preserves_source=True,relevant_policy_restore_advances_epoch=True,planning_pause_refused=True,resume_recovers=True,scope_epoch_changed=True,actual_public_worker=True,canonical_producer=True,preview_stable=True,authority_and_stale_refused=True,changed_report_refused=True,changed_declared_input_changes_source=True,missing_inputs_refused=True,exact_restore_recovers=True,no_control_execution=True,no_work_mutation=True,task_accepted=False)

def main():
    """Bind exact helpers/default binary and preserve original failed assertions.

    No global watchdog or automatic retry is installed. Individual product
    policies retain their original persisted deadlines and budgets.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve()
    sources=list(dict.fromkeys([Path(__file__),Path(native.__file__),Path(public.__file__),Path(fixture.__file__),Path(public.feedback.__file__),Path(public.mission.__file__),Path(public.feedback.native.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [public.request,public.feedback.fresh_work,public.feedback.ledger,public.feedback.apply_policy]]))
    inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        result=scenario(binary,out);assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(p).read_bytes())==h for p,h in inputs['sources'].items());fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
