"""Public-CLI campaign: prepare is unpaid; run starts explicitly authorized calls.

No direct database access. No claim/decide/retry/accept mutation is provided.
The observer is a qualification report, not a replacement for engine acceptance.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid
from controlled_business_fault import CHECK, digest

LIMITS = {'planning': 12, 'producers': 2, 'reviews': 4}
ACTIVE = {'queued', 'starting', 'running', 'stopping'}


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')
    path.chmod(0o600)


def cli(manifest, args, data=None):
    allowed = {('init',), ('work','create'), ('work','show'), ('planning','enable'),
               ('planning','history'), ('agent','list'), ('agent','stop'),
               ('mission','start'), ('mission','stop')}
    if tuple(args[:min(2,len(args))]) not in allowed:
        raise ValueError('campaign command outside setup/observation/stop allowlist')
    p = subprocess.run([manifest['engine'],'--root',manifest['store'],'--json',*args]
                       + (['--input','-'] if data is not None else []),
                       input=json.dumps(data) if data is not None else None,
                       text=True,capture_output=True,timeout=60)
    with Path(manifest['journal']).open('a') as stream:
        stream.write(json.dumps({'at':time.time(),'args':args,'request':data,'exit':p.returncode})+'\n')
    if p.returncode:
        raise RuntimeError(' '.join(args)+': '+p.stdout+p.stderr)
    return json.loads(p.stdout) if p.stdout.strip() else None


def change(m,args,w,**fields):
    value=cli(m,args,dict(schema_version=1,event_id='trial-'+uuid.uuid4().hex,
                        expected_revision=w.get('revision',0),**fields))
    return value.get('work',value)


def frozen(m):
    for name, expected in m['frozen_files'].items():
        if digest(Path(name).read_bytes()) != expected:
            raise ValueError('frozen campaign artifact changed: '+name)


def prepare(destination, engine, providers, provider_name):
    dest=Path(destination).resolve()
    dest.mkdir(mode=0o700)  # Never reuse/reset an existing trial.
    source_engine=Path(engine).resolve(strict=True)
    binary=dest/'swarm';shutil.copy2(source_engine,binary)
    store=dest/'store';store.mkdir()
    tools=dest/'tools';tools.mkdir()
    for name in ['controlled_business_fault.py','autonomy_worker_adapter.py']:
        shutil.copy2(Path(__file__).with_name(name),tools/name)
    m={'schema_version':1,'engine':str(binary),'engine_sha256':digest(binary.read_bytes()),
       'store':str(store),'journal':str(dest/'cli.jsonl'),'limits':LIMITS,
       'created_at':time.time(),'evidence_dir':str(dest/'injection'),
       'engine_build_info':subprocess.check_output(['go','version','-m',str(binary)],text=True)}
    cli(m,['init'])
    configured=json.loads(Path(providers).read_text())['providers'][provider_name]
    if Path(configured['command']).name != 'claude':
        raise ValueError('this trial adapter currently requires the Claude connector')
    if configured.get('preflight_args') or configured.get('preflight_required'):
        raise ValueError('custom provider preflight needs an explicit trial adapter; not bypassed')
    project=store/'project';project.mkdir()
    def git(*args):
        return subprocess.check_output(['git','-c','user.name=Trial','-c','user.email=trial@localhost',*args],cwd=project,text=True,stderr=subprocess.DEVNULL).strip()
    git('init');(project/'.gitignore').write_text('__pycache__/\n*.pyc\n')
    (project/'README.md').write_text('Implement feature.py:is_expected(value). Return True only for the string expected. All other values return False. Write your task report in docs/<task-id>.md.\n')
    git('add','.');git('commit','-m','Disposable business requirement')
    m['project_base']=git('rev-parse','HEAD');m['project']=str(project)
    w=change(m,['work','create'],{},title='Qualification autonomie — faute métier contrôlée',
        objective='Déléguer ce besoin à un sous-planificateur responsable. Il définit une seule tâche de production : feature.py expose is_expected(value), booléen vrai seulement pour la chaîne expected. Le contrôleur impose les tests. Après un refus, le responsable décide une correction dans les limites existantes. Chaque responsable ferme son périmètre après validation réelle et fermeture de ses enfants. Le rapport doit être docs/<task-id>.md. Aucun autre objectif.',
        scope='Dépôt jetable ; une exigence métier et une tâche ; le banc injecte une faute une seule fois après la première production correcte.',
        criteria=['is_expected renvoie exactement True pour expected et False pour wrong, chaîne vide et None'],next='Définir la délégation nécessaire puis laisser le responsable décider la tâche.')
    m['work']=w['id']
    adapter=dest/'adapter.json'
    write(adapter,dict(engine=str(binary),engine_sha256=m['engine_sha256'],store=str(store),
        work=w['id'],copies_root=str(store/'.swarm/managed'/w['id']/'copies'),
        evidence_dir=m['evidence_dir'],producer=configured['command']))
    worker=tools/'claude'
    worker.write_text('#!'+sys.executable+'\nimport sys\nsys.path.insert(0,'+repr(str(tools))+')\nfrom autonomy_worker_adapter import run\ntry:\n code=run('+repr(str(adapter))+',sys.argv[1:],sys.stdin.buffer.read())\nexcept Exception as e:\n print(str(e),file=sys.stderr);code=78\nraise SystemExit(code)\n');worker.chmod(0o700)
    original=dict(configured);wrapped=dict(configured);wrapped['command']=str(worker)
    config=store/'.swarm/providers.json';write(config,{'schema_version':1,'providers':{'trial-planner':original,'trial-worker':wrapped}})
    m['control_command']=['python3','-B','-c',CHECK,'feature.py']
    w=change(m,['planning','enable',w['id']],w,provider='trial-planner',max_tasks=1,
        max_decisions=20,max_activations=12,max_review_calls=4,
        repository={'path':str(project),'committed_only':True},
        checks={'req-1':[{'id':'business-check','command':m['control_command'],'criteria':[1],
                         'justification':'Contrôle métier figé hors des fichiers de production','timeout_seconds':10}]})
    m['profile']={'provider':'trial-worker','role':'worker','workspace':str(project),
                  'capture_output':True,'timeout_seconds':300,'limits':{'max_tool_calls':min(100,(configured.get('limits') or {}).get('max_tool_calls') or 100)}}
    m['frozen_files']={str(p):digest(p.read_bytes()) for p in
        [binary,adapter,config,worker,tools/'controlled_business_fault.py',tools/'autonomy_worker_adapter.py',Path(__file__).resolve(),Path(__file__).with_name('controlled_business_fault.py').resolve()]}
    write(dest/'prepared-work.json',w);write(dest/'campaign.json',m)
    return m


def assess(m, snapshot, history, agents, injection, failures):
    """Fail closed on missing proof, even if the task and scopes look finished."""
    missing=[]
    def need(condition,reason):
        if not condition: missing.append(reason)
    w=snapshot.get('work',snapshot);p=w.get('planning',{});tasks=w.get('tasks',[])
    need(w.get('id')==m['work'],'wrong work')
    need(p.get('max_activations')==12 and p.get('max_tasks')==1 and p.get('reviewer',{}).get('max_calls')==4,'configured limits differ')
    need(0<p.get('activations',0)<=12 and 0<p.get('reviewer',{}).get('calls',0)<=4,'planning/review consumption missing or exceeded')
    need(len(tasks)==1 and len(agents)==2,'one task and two producers required')
    need(all(a.get('status')=='completed' and a.get('role')=='worker' for a in agents),'producer not completed')
    scopes=p.get('scopes',[])
    need(len(scopes)>=2 and all(s.get('state')=='closed' for s in scopes),'responsibility scopes not closed')
    ops=[op for h in history for op in h.get('operations',[])]
    need({'delegate','task','retry','close'} <= {op.get('kind') for op in ops},'real decision trajectory incomplete')
    need(bool(history) and all(h.get('id','').startswith('planning-claim-') and h['id'].endswith('-decision') for h in history),'non-engine planning decision')
    need(any(a.get('id')==injection.get('agent') and a.get('attempt_id')==injection.get('attempt') for a in agents),'injection producer identity missing')
    need(injection.get('state')=='injected' and injection.get('work')==m['work'] and injection.get('precheck_exit')==0 and injection.get('postcheck_exit',0)!=0,'controlled injection missing')
    matching=[f for f in failures if f.get('producer')==injection.get('agent') and f.get('attempt')==injection.get('attempt')]
    need(any(f.get('result',{}).get('executed') and not f['result'].get('passed') and f['result'].get('exit_code',0)!=0 and f.get('control',{}).get('command')==m['control_command'] for f in matching),'engine refusal of injected attempt missing')
    prohibited={'review.retry','task.corrective-recovery','task.attempt-extension','planning.revoke'}
    need(not any(e.get('kind') in prohibited for e in snapshot.get('events',[])),'external recovery or pause observed')
    if len(tasks)==1:
        t=tasks[0];r=t.get('independent_review',{});v=t.get('automatic_validation',{})
        need(t.get('status')=='accepted' and snapshot.get('validation',{}).get('tasks',{}).get(t['id'],{}).get('fresh') is True,'fresh engine acceptance missing')
        need(len(t.get('attempts',[]))==2 and not t.get('recovered_result') and not t.get('corrective_recovery'),'attempt history or external repair mismatch')
        need(r.get('state')=='passed' and v.get('state')=='accepted','review or controls not accepted')
        sha=r.get('candidate_commit')
        need(bool(sha) and sha==v.get('candidate_sha')==p.get('repository',{}).get('candidate_commit'),'review/control/publication SHA mismatch')
        need(bool(r.get('attempt')) and bool(r.get('producer')) and r.get('attempt')==v.get('attempt_id') and r.get('producer')==v.get('producer_agent_id'),'review/control attribution mismatch')
        need(any(a.get('id')==r.get('producer') and a.get('attempt_id')==r.get('attempt') for a in agents),'accepted producer identity missing')
        need(r.get('reviewer')=='reviewer://trial-planner','independent reviewer identity missing')
        controls=v.get('controls',[])
        need(bool(controls) and all(c.get('executed') and c.get('passed') and c.get('exit_code')==0 for c in controls) and any(c.get('command')==m['control_command'] for c in controls),'accepted business control missing')
        attempts=t.get('attempts',[])
        need(len(attempts)==2 and attempts[0].get('id')==injection.get('attempt') and attempts[-1].get('id')==r.get('attempt'),'injection/correction attempt order mismatch')
        need(r.get('attempt')!=injection.get('attempt'),'injected attempt incorrectly accepted')
    return {'status':'PASS' if not missing else 'INCOMPLETE','missing':missing}


def observe(m):
    frozen(m)
    snap=cli(m,['work','show',m['work']]);history=cli(m,['planning','history',m['work']])
    rows=cli(m,['agent','list',m['work']])['agents'];agents=[a.get('agent',a) for a in rows]
    ip=Path(m['evidence_dir'])/'injection.json';injection=json.loads(ip.read_text()) if ip.exists() else {}
    diag=Path(m['store'])/'.swarm/managed'/m['work']/'diagnostics'
    failures=[json.loads(f.read_text()) for f in diag.glob('control-failure-*.json')]
    result=assess(m,snap,history,agents,injection,failures)
    if result['status']=='PASS':
        w=snap['work'];t=w['tasks'][0];r=t['independent_review'];root=Path(m['store'])
        for key in ('receipt','context'):
            path=(root/r[key]).resolve()
            if root.resolve() not in path.parents or digest(path.read_bytes())!=r[key+'_sha256']:
                raise ValueError('review evidence hash mismatch')
        for f in failures:
            if f.get('producer')==injection['agent'] and f.get('attempt')==injection['attempt']:
                data=base64.b64decode(f['output_base64'])
                if digest(data)!=f['result']['output_sha256']:
                    raise ValueError('failed control output hash mismatch')
                bare=root/'.swarm/managed'/m['work']/'repository.git'
                raw=subprocess.check_output(['git','--git-dir',str(bare),'show',f['candidate_commit']+':feature.py'])
                if digest(raw)!=injection['after_sha256']:
                    raise ValueError('engine did not test the injected function')
    return result,dict(snapshot=snap,history=history,agents=agents,injection=injection,failures=failures)


def run(directory, seconds=1200):
    dest=Path(directory).resolve();m=json.loads((dest/'campaign.json').read_text());frozen(m)
    # An interrupted campaign cannot silently start a new paid trajectory.
    with (dest/'run-started.json').open('x') as f: json.dump({'at':time.time(),'limits':LIMITS},f)
    log=(dest/'server.log').open('ab',buffering=0)
    server=None;result={'status':'FAIL','missing':['campaign did not complete']}
    try:
        cli(m,['mission','start',m['work']],m['profile'])
        server=subprocess.Popen([m['engine'],'--root',m['store'],'web','127.0.0.1:0'],stdout=log,stderr=log)
        deadline=time.monotonic()+seconds
        last_revision=None;quiet_since=time.monotonic()
        while time.monotonic()<deadline:
            if server.poll() is not None: raise RuntimeError('trial conductor stopped')
            result,evidence=observe(m);write(dest/'observed.json',evidence);write(dest/'result.json',result)
            if result['status']=='PASS':break
            w=evidence['snapshot']['work'];p=w['planning']
            if w['revision']!=last_revision:
                last_revision=w['revision'];quiet_since=time.monotonic()
            busy=any(a.get('status') in ACTIVE for a in evidence['agents']) or any(s.get('holder') for s in p['scopes']) or any(t.get('independent_review',{}).get('state')=='running' for t in w['tasks'])
            exhausted=p.get('activations',0)>=12 or any(t.get('status')=='blocked' and len(t.get('attempts',[]))>=2 for t in w['tasks'])
            if p.get('failure') or (exhausted and not busy and time.monotonic()-quiet_since>=30):
                break
            time.sleep(10)
        if result['status']!='PASS': result['status']='FAIL'
    except Exception as exc:
        result={'status':'FAIL','missing':[str(exc)]}
    finally:
        # Stop is not a recovery decision. Preserve agents/processes for diagnosis
        # if shutdown cannot be confirmed; never abandon them silently.
        try:
            cli(m,['mission','stop',m['work']])
            rows=cli(m,['agent','list',m['work']])['agents']
            for row in rows:
                a=row.get('agent',row)
                if a['status'] in ACTIVE: cli(m,['agent','stop',a['id']])
            deadline=time.monotonic()+30
            while True:
                rows=cli(m,['agent','list',m['work']])['agents']
                live=any(row.get('agent',row).get('status') in ACTIVE for row in rows)
                current=cli(m,['work','show',m['work']])['work']
                reviewing=any(t.get('independent_review',{}).get('state')=='running' for t in current['tasks'])
                if not live and not reviewing:break
                if time.monotonic()>deadline:raise RuntimeError('agents or reviews still active; server retained')
                time.sleep(1)
            if server is not None: server.terminate();server.wait(timeout=30)
        except Exception as exc:
            result={'status':'FAIL','missing':['cleanup not confirmed: '+str(exc)],'previous_result':result}
        log.close();write(dest/'result.json',result)
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='action',required=True)
    prep=sub.add_parser('prepare');prep.add_argument('directory');prep.add_argument('--engine',required=True);prep.add_argument('--providers',required=True);prep.add_argument('--provider',default='claude')
    start=sub.add_parser('run');start.add_argument('directory')
    args=parser.parse_args()
    if args.action=='prepare':
        m=prepare(args.directory,args.engine,args.providers,args.provider)
        print(json.dumps({'status':'PREPARED_NOT_STARTED','work':m['work'],'limits':LIMITS}))
    else:
        result=run(args.directory);print(json.dumps(result));raise SystemExit(result['status']!='PASS')
