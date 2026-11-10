#!/usr/bin/env python3
"""Read a locked conversion via public CLI/API; never authorize a departure.

Owned fixtures only; compare complete SQL before/after and preserve refusals,
recovery and original channels. No live Root, provider or global watchdog.
"""
import argparse,json,sqlite3
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Require CLI/API equality and current revisions without losing sidecars."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Retain each named observation; no empty/zero-test success."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def command(p,action,event,**extra):
        """Submit an explicit event bound to the current preparation revision."""
        return d.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra))
    d.ok(['init']);d.settings(preparation_write_enabled='1');p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='review-create',expected_revision=0,title='Review without release',text='Need'));unconverted=p
    p=command(p,'save','review-brief',document='brief',text='Brief');p=command(p,'adopt-brief','review-adopt',sha256=p['documents']['brief']['sha256']);p=command(p,'save','review-plan',document='plan',text=json.dumps(plan()));p=command(p,'validate-plan','review-validate',sha256=p['documents']['plan']['sha256']);p=command(p,'create-missions','review-convert',sha256=p['documents']['plan']['sha256'],expected_work_revision=0)
    before=d.database();rc,stdout,stderr=d.call(['prepare','release-review',p['id']])
    if a.expect_unported:
        check('release-review-baseline-cli-refuses',rc!=0 and before==d.database())
        fresh=d.ok(['prepare','create'],dict(version=1,action='create',event_id='baseline-unconverted',expected_revision=0,title='Unconverted baseline',text='Need'));before=d.database();server=native.Server(d.binary,d.root,out,'baseline-release-review');primary=None
        try:server.request('/api/v1/preparations/release-review?id='+fresh['id'],status=409,observe=False);check('release-review-baseline-http409',before==d.database())
        except BaseException as error:primary=error
        finally:native.finish(server,primary)
        print('RELEASE_REVIEW_BASELINE_PASS 2');return
    check('release-review-cli-available',rc==0 and not stderr);view=json.loads(stdout);work=d.ok(['work','show',p['work_id']])['work'];check('release-review-cli-exact-shape',view==dict(preparation=d.ok(['prepare','show',p['id']]),work=work));check('release-review-locks-not-released',all(t.get('launch_held') for t in view['work']['tasks']) and not view['preparation']['conversion'].get('released_at'));check('release-review-does-not-write',d.database()==before)
    fresh=d.ok(['prepare','create'],dict(version=1,action='create',event_id='review-unconverted',expected_revision=0,title='Unconverted',text='Need'));before=d.database();rc,_,err=d.call(['prepare','release-review',fresh['id']]);check('release-review-unconverted-refuses',rc!=0 and json.loads(err)['failure']['code']=='preparation-stale-plan' and before==d.database());check('release-review-unconverted-recovery',d.ok(['prepare','release-review',p['id']])==view)
    server=native.Server(d.binary,d.root,out,'release-review');server.capture_async(out/'observer.raw');primary=None
    try:
        path='/api/v1/preparations/release-review?id='+p['id'];check('release-review-http-cli-equal',server.request(path,observe=False)==view);server.request('/api/v1/preparations/release-review?id='+fresh['id'],status=409,observe=False);check('release-review-unconverted-http-refuses',before==d.database());server.request(path+'&id='+p['id'],status=400,observe=False);check('release-review-invalid-query-no-write',before==d.database())
        updated=d.ok(['task','update',work['id']],dict(schema_version=1,event_id='review-task-edit',expected_revision=work['revision'],id=work['tasks'][0]['id'],title='Current changed task',next='Current instruction'));current=d.ok(['work','show',work['id']])['work'];next_view=d.ok(['prepare','release-review',p['id']]);check('release-review-current-work-revision',next_view['work']==current and current['revision']==work['revision']+1 and current['tasks'][0]['title']=='Current changed task');check('release-review-current-http-cli-equal',server.request(path,observe=False)==next_view)
        # Valid witness immediately precedes each owned fixture corruption.
        before=d.database()
        with sqlite3.connect(d.root/'.swarm/state.db') as db:raw=db.execute('SELECT body FROM preparations WHERE id=?',(p['id'],)).fetchone()[0]
        broken=json.loads(raw);broken['conversion']['work_id']='w-unrelated';d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(broken),p['id']));fault=d.database();rc,_,err=d.call(['prepare','release-review',p['id']]);check('release-review-scope-mismatch-refuses',rc!=0 and json.loads(err)['failure']['code']=='preparation-release-scope-unported' and fault==d.database());d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(raw,p['id']));check('release-review-scope-recovery',d.ok(['prepare','release-review',p['id']])==next_view and d.database()==before)
        # Unavailable local method returns readable data plus a sidecar. A later
        # scope refusal must retain that cause instead of dropping it as GLM proposed.
        local=d.root/'.claude/skills/apex/SKILL.md';local.parent.mkdir(parents=True,exist_ok=True);local.symlink_to('missing-method');rc,_,err=d.call(['prepare','release-review',p['id']]);check('release-review-unavailable-method-readable',rc==0 and not err and not json.loads(_)['preparation']['plan_ready']);d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(broken),p['id']));fault=d.database();rc,_,err=d.call(['prepare','release-review',p['id']]);error=json.loads(err);check('release-review-method-cause-retained-on-refusal',rc!=0 and error['failure']['code']=='root-cleanup' and 'preparation-release-scope-unported' in error['failure'].get('causes',[]) and len(error['failure'].get('causes',[]))>1 and any('preparation-method' in c for c in error['failure']['causes']) and fault==d.database());d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(raw,p['id']));local.unlink();check('release-review-method-recovery',d.ok(['prepare','release-review',p['id']])==next_view)
        with sqlite3.connect('file:'+str(d.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:count=db.execute('SELECT count(*) FROM agents').fetchone()[0]
        check('release-review-no-agent-rows',count==0)
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0));print('RELEASE_REVIEW_PASS '+str(len(checks)))
if __name__=='__main__':main()
