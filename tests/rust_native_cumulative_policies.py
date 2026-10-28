#!/usr/bin/env python3
"""Check saved operator policy consumption after real cumulative publication.

Public preview/apply/reopen changes only the owned inert Root's saved settings;
all values are restored through the same API, keeping history. No provider,
positive SQL seed, Work rewrite, budget reset or watchdog is introduced.
"""
import argparse,json,os,sqlite3,traceback,uuid
from pathlib import Path
import rust_native_cumulative_publication as cumulative
from rust_native_task_validation import invoke


def run(binary,profile,out,published_from):
    """Verify unused-default stability and actual saved control-limit refusal.

    Requires an actual successful two-task cumulative publication. Witnesses
    precede policy changes, restoration runs even after refusal/check failure.
    Work/finance/provider markers remain unchanged; settings history intentionally
    records both change and restore. All original channels/errors are retained.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False);f=cumulative.fixture
    source=json.loads((published_from/'result.json').read_bytes());assert source['status']=='PASS' and source['cumulative_publication'] and not source['paid_provider']
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and qualified['sha256']==f.digest(binary.read_bytes())
    root=Path(source['root']);work=source['work'];assert root.is_absolute() and root.name.startswith('swarm-model83-')
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'));namespace=uuid.uuid4().hex
    def call(label,args,value=None,success=True):
        """Keep exact public command responses; a refusal never becomes empty."""
        code,result=invoke(binary,root,out,label,args,value,environment);assert (code==0)==success,(label,result);return result
    before=call('before',['work','show',work])['work'];finance=cumulative.baseline.workers.publication.native.finance(root,work)
    def recorded():
        """Read exact Work bytes even while its public evidence projection refuses.

        Only this identified inert fixture is opened read-only; no successful
        canonical state is synthesized or changed by the test observer.
        """
        connection=sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:return connection.execute('SELECT * FROM works WHERE id=?',(work,)).fetchall()
        finally:connection.close()
    original_recorded=recorded()
    original=call('settings-original',['engine-settings','show']);values=original['values'];checks=[]
    tasks=[t for t in before['tasks'] if t['status']=='accepted'];assert len(tasks)==2 and all(t['validation_policy']['controls'] for t in tasks)
    assert all(int(c['timeout_seconds'])>0 for t in tasks for c in t['validation_policy']['controls'])
    request=dict(schema_version=1,event_id='policy-observe-'+namespace,expected_revision=before['revision'],task_id=tasks[0]['id'])
    def valid(label):
        """Read actual current candidate evidence without granting a future action."""
        result=call(label,['task','baseline-preview',work],request);assert result['baseline_verified'] and len(result['tasks'])==2 and result['candidate_commit']==source['candidate_commit'];return result
    def apply(label,changed):
        """Persist exact entered values through preview/apply/reopen, not private IO."""
        doc=call(label+'-show',['engine-settings','show']);intent=dict(schema_version=doc['schema_version'],event_id=label+'-'+namespace,expected_revision=doc['revision'],values=changed,reason='Verify saved cumulative control policy consumption',preview_token='')
        preview=call(label+'-preview',['engine-settings','preview'],intent);assert call(label+'-preview-readonly',['engine-settings','show'])==doc
        intent['preview_token']=preview['preview_token'];call(label+'-apply',['engine-settings','apply'],intent)
        reopened=call(label+'-reopen',['engine-settings','show']);assert reopened['values']==changed and len(reopened['history'])>len(doc['history'])
    def restore(label):
        """Restore saved values only when needed, retaining the complete history."""
        if call(label+'-inspect',['engine-settings','show'])['values']!=values:apply(label,values)
    valid('default-witness')
    with cumulative.restoration(lambda:restore('default-restore')):
        changed=dict(values,validation_default_timeout_seconds=str(int(values['validation_default_timeout_seconds'])+1))
        apply('unused-default',changed);valid('unused-default-current-plan-unchanged')
        assert call('unused-default-work',['work','show',work])['work']==before
    valid('default-restored');checks.append('saved-unused-default-does-not-invalidate-explicit-control-timeouts')
    valid('limit-witness')
    with cumulative.restoration(lambda:restore('control-limit-restore')):
        changed=dict(values,validation_max_controls=str(min(len(t['validation_policy']['controls']) for t in tasks)-1))
        apply('actual-control-limit',changed)
        denied=call('actual-control-limit-refused',['task','baseline-preview',work],request,False)
        assert denied['failure']['causes'] and recorded()==original_recorded
    valid('limit-restored');checks.append('saved-operator-control-limit-consumed-and-restored-with-history')
    assert cumulative.baseline.workers.publication.native.finance(root,work)==finance and call('final-work',['work','show',work])['work']==before
    f.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,provider_started=False,paid_provider=False,work_unchanged=True,settings_values_restored=True,settings_history_preserved=True))
    print('CUMULATIVE_SAVED_POLICIES_VERIFIED '+str(len(checks)),flush=True)


def main():
    """Freeze actual helper/binary inputs and preserve errors as failed evidence."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output','published-from']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();out=args.output.resolve();f=cumulative.fixture
    inputs=dict(binary_sha256=f.digest(args.binary.read_bytes()),profile_sha256=f.digest(args.profile.read_bytes()),sources=cumulative.baseline.workers.publication.native.inspection.native.helper_inputs(Path(__file__)))
    try:
        run(args.binary.resolve(),args.profile.resolve(),out,args.published_from.resolve())
        assert f.digest(args.binary.read_bytes())==inputs['binary_sha256'] and all(f.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        f.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():f.publish(out/'inputs.json',inputs);f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
