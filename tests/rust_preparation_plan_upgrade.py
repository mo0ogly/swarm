#!/usr/bin/env python3
"""Historical v42 Root -> explicit Admin v43 caps -> plan validator consumer.

Old/new Rust binaries operate on a newly owned fixture. Missing policy refuses
without mutation; browser edits persist and affect actual CLI validation, with
history preserved. No live Root, provider or global watchdog.
"""
import argparse
import json
import subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan, task
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Retain channels and named assertions; finish owned HTTP server on error."""
    parser=argparse.ArgumentParser();parser.add_argument('--old-binary',required=True);parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.old_binary).resolve(),out);checks=[]

    def check(name,value):
        """Require distinct observable checks, preserving their actual results."""
        assert value,name;assert name not in checks,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)

    d.ok(['init']);d.settings(preparation_list_limit='2');old=d.ok(['engine-settings','show'])
    check('plan-upgrade-v42-control',old['schema_version']==42 and 'preparation_plan_max_bytes' not in old['values'])
    p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='upgrade-plan-create',expected_revision=0,text='need'));pid=p['id']
    p=d.ok(['prepare','save',pid],dict(version=1,action='save',event_id='upgrade-brief-save',preparation_id=pid,expected_revision=1,document='brief',text='brief'))
    p=d.ok(['prepare','adopt-brief',pid],dict(version=1,action='adopt-brief',event_id='upgrade-adopt',preparation_id=pid,expected_revision=2,sha256=p['documents']['brief']['sha256']))
    p=d.ok(['prepare','save',pid],dict(version=1,action='save',event_id='upgrade-plan-save',preparation_id=pid,expected_revision=3,document='plan',text=json.dumps(plan())))
    intent=dict(version=1,action='validate-plan',event_id='upgrade-verdict',preparation_id=pid,expected_revision=p['revision'],sha256=p['documents']['plan']['sha256'])
    work=d.ok(['work','create'],dict(schema_version=1,event_id='upgrade-admin-work',expected_revision=0,title='Owned upgrade',objective='Verify Admin consumer',scope='No execution',criteria=['History preserved']))['work']['id']
    d.binary=Path(a.binary).resolve();before=d.database();code,stdout,stderr=d.call(['prepare','validate-plan',pid],intent)
    check('plan-upgrade-missing-policy-refuses',code==2 and not stdout and json.loads(stderr)['failure']['code']=='settings-preparation-plan-not-configured' and d.database()==before)
    check('plan-upgrade-draft-still-readable',d.ok(['prepare','show',pid])['revision']==4)
    server=native.Server(d.binary,d.root,out,'plan-upgrade');server.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(address=server.address,key=server.key,root=str(d.root),work=work,output=str(out))
        with (out/'browser.stdout').open('xb') as o,(out/'browser.stderr').open('xb') as e:r=subprocess.run(['node','tests/rust_preparation_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=o,stderr=e)
        check('plan-upgrade-explicit-Admin-adoption',r.returncode==0)
        new=d.ok(['engine-settings','show']);check('plan-upgrade-history-preserved',new['schema_version']==43 and new['history'][:len(old['history'])]==old['history'] and len(new['history'])==len(old['history'])+1)
        check('plan-upgrade-browser-cap-persisted',new['values']['preparation_plan_max_tasks']=='1')
        p=d.ok(['prepare','validate-plan',pid],intent);check('plan-upgrade-same-event-now-valid',p['plan_ready'] and p['revision']==5)
        two={**plan(),'tasks':[task('t1'),task('t2',['t1'])]}
        p=d.ok(['prepare','save',pid],dict(version=1,action='save',event_id='upgrade-two-tasks',preparation_id=pid,expected_revision=p['revision'],document='plan',text=json.dumps(two)))
        intent={**intent,'event_id':'upgrade-two-verdict','expected_revision':p['revision'],'sha256':p['documents']['plan']['sha256']}
        before=d.database();code,stdout,stderr=d.call(['prepare','validate-plan',pid],intent)
        check('plan-upgrade-real-Admin-cap-consumer',code==2 and not stdout and json.loads(stderr)['failure']['code']=='preparation-invalid-plan' and d.database()==before)
        d.settings(preparation_plan_max_tasks='2');p=d.ok(['prepare','validate-plan',pid],intent);check('plan-upgrade-consumer-same-event-recovery',p['plan_ready'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0));print('PREPARATION_PLAN_UPGRADE_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
