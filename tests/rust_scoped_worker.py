#!/usr/bin/env python3
"""Exercise an explicit native worker on retained hierarchical fixture data.

Planning creation/automatic dispatch/delegation is not established: seed only this
owned temporary Work with a Go-shaped scope plan. All admission, process, report,
budget and reads then use public native CLI operations. One inert program emits
facts and a report; no model, account, live root, independent review or acceptance.
"""
import argparse,json,os,sqlite3,traceback,shutil,subprocess
from pathlib import Path
import rust_agent_model_route as m
import rust_manual_mission as mission
import rust_native_completion_recovery as recovery


def seed(f,seeder):
    """Create canonical retained fixture only, without editing a live mission.

    The minimal reviewer provider's exact Go/native wire bytes are retained; its
    digest is a source/config consistency check, never actor authentication.
    """
    provider=dict(limits=dict(silence_seconds=0,tool_seconds=0,max_tool_calls=0,max_repeated_calls=0,max_consecutive_errors=0),command=str(f.root/'codex'),args=None,env_allow=None)
    canonical=json.dumps(provider,ensure_ascii=False,separators=(',',':')).encode()
    m.publish(f.output/(f.label+'-reviewer-provider-wire.json'),canonical)
    m.publish(f.providers,dict(schema_version=1,providers=dict(fixture=f.provider,reviewer=provider)))
    db=sqlite3.connect(f.root/'.swarm/state.db')
    try:
        row=db.execute('SELECT body FROM works WHERE id=?',(f.work,)).fetchone();w=json.loads(row[0])
        w.update(title='ROOT-PRIVATE-TITLE',objective='ROOT-PRIVATE-OBJECTIVE',scope='ROOT-PRIVATE-BRIEF',summary='ROOT-PRIVATE-CHECKPOINT',criteria=['OWN-REQUIREMENT','SIBLING-PRIVATE-REQUIREMENT'])
        t=w['tasks'][0];t.update(scope_id='child',plan_role='worker',requirements=['req-1'],title='OWN-TASK',deliverable='OWN-DELIVERY',criteria=['OWN-CRITERION'],validation_policy=dict(mode='human',authorized_at='2026-10-20T00:00:00Z'))
        w['planning']=dict(version=1,max_tasks=10,max_decisions=10,max_activations=10,provider='fixture',human_review_authorized='2026-10-20T00:00:00Z',scopes=[dict(id='root',revision=1,state='ready',objective='ROOT-PRIVATE-OBJECTIVE',requirements=[]),dict(id='child',parent='root',revision=1,state='ready',objective='OWN-SCOPE',requirements=['req-1']),dict(id='sibling',parent='root',revision=1,state='ready',objective='SIBLING-PRIVATE-SCOPE',requirements=['req-2'])],reviewer=dict(provider='reviewer',provider_digest=m.digest(canonical),authorized='2026-10-20T00:00:00Z',max_calls=10))
        assert w['revision']==f.request['expected_revision']
    finally:db.close()
    candidate=f.output/(f.label+'-seeded-work.json');m.publish(candidate,w)
    m.publish(f.root/'.swarm/scoped-fixture-owner',('swarm-scoped-worker-fixture-v1\n'+str(f.root)+'\n'+f.work+'\n').encode())
    with (f.output/(f.label+'-seed.stdout')).open('xb') as out,(f.output/(f.label+'-seed.stderr')).open('xb') as err:
        result=subprocess.run([str(seeder),str(f.root),f.work,str(candidate)],stdout=out,stderr=err)
    assert result.returncode==0,'Native fixture seeder refused; inspect retained stderr'
    m.publish(f.root/'codex',mission.PROGRAM.encode());os.chmod(f.root/'codex',0o700)


def run(f,seeder):
    """Assert selected identity/scope across real admission, process and handoff.

    Private root/sibling markers must never occur in the actual delivered prompt.
    Terminal and handoff signals target only the owning scope/current attempt;
    submission adds no attempt, reviewer consumption or accepted result.
    """
    seed(f,seeder)
    for patch in [dict(role='planner'),dict(parent='claimed-parent'),dict(origin='conducteur')]:
        f.prepare(preview=True);request=dict(f.request,**patch)
        mission.reject(f,['agent','prepare',f.work],request)
    f.prepare(preview=True);catalogue=f.providers.read_bytes();wrong=json.loads(catalogue)
    wrong['providers'].pop('reviewer');m.publish(f.providers,wrong)
    mission.reject(f,['agent','prepare',f.work],f.request)
    m.publish(f.providers,catalogue);f.prepare(preview=True)
    prepared=f.prepare();agent=prepared['agent'];prompt=agent['prompt']
    for own in ['OWN-SCOPE','OWN-TASK','OWN-DELIVERY','OWN-CRITERION','OWN-REQUIREMENT',f.work,'session',agent['attempt_id']]:assert own in prompt,own
    for private in ['ROOT-PRIVATE-TITLE','ROOT-PRIVATE-OBJECTIVE','ROOT-PRIVATE-BRIEF','ROOT-PRIVATE-CHECKPOINT','SIBLING-PRIVATE-REQUIREMENT','SIBLING-PRIVATE-SCOPE']:assert private not in prompt,private
    f.run_and_assert(prepared)
    w=f.ok(['work','show',f.work])['work'];attempt=w['tasks'][0]['attempts'][-1]['id'];assert attempt==agent['attempt_id']
    ended=[e for e in w['planning'].get('inbox',[]) if e['kind']=='attempt_ended'];assert len(ended)==1 and ended[0]['scope']=='child' and ended[0]['attempt']==attempt
    assert ended[0]['id']=='plan-'+m.digest(json.dumps(['attempt-ended',attempt],separators=(',',':')).encode())
    scopes={s['id']:s for s in w['planning']['scopes']};assert scopes['child']['state']=='ready'
    assert w['tasks'][0]['status']=='blocked' and w['planning']['reviewer'].get('calls',0)==0
    r=mission.handoff(f);result=f.ok(['task','submit',f.work],r);w=f.ok(['work','show',f.work])['work']
    assert w['tasks'][0]['status']=='submitted' and len(w['tasks'][0]['attempts'])==1
    signals=[e for e in w['planning']['inbox'] if e['kind']=='handoff'];assert len(signals)==1
    signal=signals[0];assert signal['scope']=='child' and signal['task']=='task' and signal['attempt']==attempt
    assert signal['id']=='plan-'+m.digest(json.dumps(['report','task',attempt],separators=(',',':')).encode())
    assert signal['handoff']==dict(path='handoff.md',sha256=r['report_sha256']) and signal['artifacts']==[signal['handoff']]
    assert w['planning']['reviewer'].get('calls',0)==0 and tuple(f.reservation())==('estimated',1.0)
    repeat=f.ok(['task','submit',f.work],r);assert f.ok(['work','show',f.work])['work']==w
    return dict(scope='child',agent='session',attempt=attempt,task_status='submitted',accepted=False,signals=2,provider_rerun=False)



def terminal(f,outcome):
    """Observe actual outcome, current scope signal and conservative reservation."""
    w=f.ok(['work','show',f.work])['work'];a=f.ok(['agent','show','session'])['agent']
    t=w['tasks'][0];assert a['status']==outcome and t['status']=='blocked'
    assert t['attempts'][-1]['status']==outcome and a.get('native_execution_result') is not None
    signals=[e for e in w['planning'].get('inbox',[]) if e['kind']=='attempt_ended']
    assert len(signals)==1 and signals[0]['scope']=='child' and signals[0]['attempt']==a['attempt_id']
    assert tuple(f.reservation())==('estimated',1.0) and w['planning']['reviewer'].get('calls',0)==0
    return outcome


def cancelled(f,seeder):
    """Cancel a genuinely unstarted scoped worker, without running a provider."""
    seed(f,seeder);prepared=f.prepare();assert not (f.root/'model-received.json').exists()
    result=f.ok(['agent','reconcile','session']);assert result['cancelled_before_start'] is True
    assert result['agent']['status']=='interrupted' and tuple(f.reservation())==('released',1.0)
    w=f.ok(['work','show',f.work])['work'];t=w['tasks'][0]
    assert t['status']=='blocked' and len(t['attempts'])==1 and t['attempts'][0]['status']=='interrupted'
    signals=[e for e in w['planning'].get('inbox',[]) if e['kind']=='attempt_ended'];assert len(signals)==1 and signals[0]['scope']=='child'
    before=f.snapshot();f.ok(['agent','reconcile','session']);assert f.snapshot()==before
    assert not (f.root/'model-received.json').exists()
    return dict(outcome='cancelled_before_start',provider_started=False,reservation='released')


def failed(f,seeder):
    """A real nonzero local provider exit settles failed, never empty success."""
    seed(f,seeder);m.publish(f.root/'codex',(mission.PROGRAM+'\nraise SystemExit(7)\n').encode());os.chmod(f.root/'codex',0o700)
    f.prepare();code,result=f.call(['agent','run','session']);assert result['agent']['status']=='failed',result
    terminal(f,'failed');assert result['agent']['exit_code']==7
    mission.reject(f,['task','submit',f.work],mission.handoff(f))
    return dict(outcome='failed',provider_exit=7,task_accepted=False)


def stopped(f,seeder):
    """Record public stop after actual provider facts, before fixture release."""
    seed(f,seeder)
    def stop():
        value=f.ok(['agent','stop','session']);assert value['desired']=='stop' and value['termination_confirmed'] is False
    code,result,_=recovery.run_held(f,False,before_release=stop)
    assert code==0 and result['agent']['status']=='interrupted',result
    terminal(f,'interrupted');return dict(outcome='interrupted',public_stop=True,task_accepted=False)


def recovered_completion(f,seeder):
    """Reapply one scoped settlement after actual provider exit and SQL refusal.

    The receipt must settle exactly once, deliver one owning-scope signal and
    preserve consumption. Removing only our trigger is the changed precondition;
    reconcile never executes the provider again or submits a nonexistent report.
    """
    seed(f,seeder);code,error,_=recovery.run_held(f,True);assert code!=0
    before=f.ok(['agent','show','session'])['agent'];assert before['status']=='running' and before.get('native_execution_result') is None
    received=(f.root/'model-received.json').read_bytes()
    value=f.ok(['agent','reconcile','session']);assert value['completion_recovered'] is True
    terminal(f,'completed');state=f.snapshot()
    f.ok(['agent','reconcile','session']);assert f.snapshot()==state and (f.root/'model-received.json').read_bytes()==received
    return dict(outcome='completion_recovered',scope='child',signals=1,idempotent=True,provider_rerun=False)

def main():
    """Preserve source/binary identity and failures; remove only successful roots."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--seeder',required=True,type=Path);p.add_argument('--cases',choices=['completed','all'],default='all');p.add_argument('--output',required=True,type=Path);a=p.parse_args();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    sources=[Path(__file__).resolve(),Path(m.__file__).resolve(),Path(mission.__file__).resolve(),Path(recovery.__file__).resolve(),a.binary.resolve(),a.seeder.resolve()];inputs={str(p):m.digest(p.read_bytes()) for p in sources};m.publish(output/'inputs.json',inputs)
    for p in sources[:4]:m.publish(output/('source-'+p.name),p.read_bytes())
    f=None
    try:
        f=m.Fixture(a.binary.resolve(),output,'scoped');result=run(f,a.seeder.resolve())
        cases=[dict(outcome='completed',result=result)]
        shutil.rmtree(f.root)
        if a.cases=='all':
            for label,case in [('cancelled',cancelled),('failed',failed),('stopped',stopped),('recovered',recovered_completion)]:
                f=m.Fixture(a.binary.resolve(),output,label);cases.append(case(f,a.seeder.resolve()));shutil.rmtree(f.root)
        assert inputs=={str(p):m.digest(p.read_bytes()) for p in sources}
        m.publish(output/'result.json',dict(status='PASS',result=result,cases=cases,source_stable=True,global_watchdog=None,planning_seed='owned fixture only, not public planning adoption'))
        print('PASS explicit native scoped worker and report handoff',flush=True)
    except BaseException as error:
        m.publish(output/'result.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,root=str(f.root) if f else None,source_stable=inputs=={str(p):m.digest(p.read_bytes()) for p in sources}));traceback.print_exc();raise

if __name__=='__main__':main()
