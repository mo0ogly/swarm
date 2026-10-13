"""Opt-in public-CLI RETEX recipes; never executed by ordinary CI.

All campaign parameters are explicit. Real calls inherit the selected configured
provider policy. Recovery starts with a labelled deterministic faulty producer;
its positive claim is untrusted test input, NOT a fabricated engine verdict.
The subsequent reviewer, supervisor and corrective worker invoke real Codex.
"""
from pathlib import Path
import argparse,datetime,hashlib,json,os,signal,subprocess,sys,time,uuid


def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def digest(raw):return hashlib.sha256(raw).hexdigest()

class Campaign:
 def __init__(self,args):
  self.args=args;self.binary=Path(args.binary).resolve();self.out=Path(args.output).resolve();self.watch=None;self.work=None
  self.out.mkdir(mode=0o700,exist_ok=args.resume_setup or args.run_prepared or args.resume_mission)
  if args.resume_setup and (self.out/'frozen.json').exists():raise ValueError('Setup is complete: resume requires an explicit replan, not replaying setup')
  self.root=self.out/'store';self.project=self.root/'project';self.project.mkdir(parents=True,exist_ok=args.resume_setup or args.run_prepared or args.resume_mission)
  self.records=self.out/'cli.jsonl';self.snapshots=self.out/'snapshots';self.snapshots.mkdir(exist_ok=args.resume_setup or args.run_prepared or args.resume_mission)
 def cli(self,parts,data=None,timeout=None):
  command=[str(self.binary),'--root',str(self.root),'--json',*parts]+(['--input','-'] if data is not None else [])
  started=utc();result=subprocess.run(command,input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=timeout or self.args.cli_seconds)
  with self.records.open('a') as f:f.write(json.dumps({'started':started,'finished':utc(),'command':command,'input':data,'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr},ensure_ascii=False)+'\n')
  if result.returncode:raise RuntimeError('CLI refused '+repr(parts)+'; original saved in cli.jsonl')
  return json.loads(result.stdout) if result.stdout.strip() else None
 def change(self,parts,work,**fields):
  value=self.cli(parts,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=work.get('revision',0),**fields))
  return value.get('work',value) if isinstance(value,dict) else value
 def git(self,*parts):return subprocess.check_output(['git',*parts],cwd=self.project,stderr=subprocess.DEVNULL,text=True).strip()
 def show(self):
  result=self.cli(['work','show',self.work['id']]);self.work=result['work'];raw=json.dumps(result,ensure_ascii=False,indent=2).encode();path=self.snapshots/(str(self.work['revision'])+'-'+digest(raw)[:12]+'.json')
  if not path.exists():path.write_bytes(raw)
  return result
 def configure(self):
  if not (self.project/'.git').exists():
   self.git('init');(self.project/'.gitignore').write_text('__pycache__/\n')
   for index in range(self.args.tasks):
    name='number' if self.args.scenario=='recovery' else f'module_{index+1}'
    (self.project/(name+'.py')).write_text('def answer():\n    return 0\n')
   self.git('add','.');self.git('commit','-m','Isolated RETEX recipe baseline')
  elif self.git('status','--porcelain'):raise ValueError('Existing original source is not clean; preserve and replan')
  self.base=self.git('rev-parse','HEAD')
  if not (self.root/'.swarm').exists():self.cli(['init'])
  configured=json.loads(Path(self.args.providers).read_text());provider=configured['providers'][self.args.provider]
  if Path(provider['command']).name!='codex':raise ValueError('This recipe requires the configured Codex adapter, never Claude.')
  if not provider.get('model_policy'):raise ValueError('An explicit configured model policy is required; no inherited CLI model.')
  self.config={'schema_version':1,'providers':{self.args.provider:provider}}
  config=self.root/'.swarm/providers.json';config.write_text(json.dumps(self.config));config.chmod(0o600)
  if self.args.scenario=='recovery':
   objective='One existing task only: recover-number. Do not create tasks or delegate. A labelled faulty producer has been seeded through public setup. Await its actual independent semantic refusal. When changes_requested identifies the wrong number value, inspect the exact received incident and decide retry for the SAME task with a NEW instruction fixing number.answer() to 42, preserving the original review/result and limits. Do not use review-retry for a semantic code defect. No manual correction or acceptance occurs after mission start. When the corrected task is accepted/fresh, emit judge end for the current cycle and candidate. While review/worker is running, acknowledge events and wait; no unnecessary new cycle.'
  else:objective='All tasks are already assigned. Do not create or delegate tasks. Await accepted/fresh results and emit judge end for the current cycle and candidate. Acknowledge arrivals without empty repeated cycles. No other objective.'
  existing=self.cli(['work','list'])
  if len(existing)>1:raise ValueError('Ambiguous existing setup; no replacement work created')
  self.work=existing[0] if existing else self.change(['work','create'],{},title='RETEX '+self.args.scenario,objective=objective,scope='Disposable Python project; no historical mission; real Codex except labelled initial fault fixture',criteria=['number.answer() returns 42'] if self.args.scenario=='recovery' else [f'module_{i+1}.answer() returns {i+1}' for i in range(self.args.tasks)],next='Observe assigned task results and finish under fresh evidence')
  checks={};tasks=[]
  for index in range(self.args.tasks):
   module='number' if self.args.scenario=='recovery' else f'module_{index+1}';value=42 if self.args.scenario=='recovery' else index+1;tid='recover-number' if self.args.scenario=='recovery' else module
   assertion=f'import {module};assert isinstance({module}.answer(),int)' if self.args.scenario=='recovery' else f'import {module};assert {module}.answer()=={value}'
   justification='Structural smoke test only; it cannot prove 42. The independent reviewer must inspect the supplied source for the semantic criterion.' if self.args.scenario=='recovery' else 'Execute the exact expected return value.'
   checks[f'req-{index+1}']=[dict(id='return-check',command=[self.args.control_program,'-B','-c',assertion],criteria=[1],justification=justification,timeout_seconds=self.args.control_seconds)]
   tasks.append(dict(id=tid,title='Implement '+module+'.py',requirements=[f'req-{index+1}'],deliverable=module+'.py and docs/'+tid+'.md',criteria=[f'{module}.answer() returns integer {value}'],next=f'Implement {module}.answer() returning {value}. Modify only that module and docs/{tid}.*. Test the exact value, declare source in delivery evidence and report actual checks. Stop after handoff; do not wait for review.'))
  if not self.work.get('planning'):self.work=self.change(['planning','enable',self.work['id']],self.work,provider=self.args.provider,max_tasks=self.args.tasks,max_decisions=self.args.decisions,max_activations=self.args.activations,max_review_calls=self.args.reviews,repository={'path':str(self.project),'committed_only':True},checks=checks)
  from organized_fixture import add_owned_tasks
  if not self.work.get('tasks'):self.work=add_owned_tasks(self.cli,self.work,tasks)
  self.profile=dict(provider=self.args.provider,role='worker',workspace=str(self.project),capture_output=True,timeout_seconds=0)
  self.cli(['profile',self.work['id']],self.profile);self.cli(['autonomy',self.work['id'],'autonome',str(self.args.slots)])
  self.show()
  frozen={'at':utc(),'binary':str(self.binary),'binary_sha256':digest(self.binary.read_bytes()),'binary_version':json.loads(subprocess.check_output([str(self.binary),'--json','version'],text=True))['binary'],'work':self.work['id'],'root':str(self.root),'project':str(self.project),'baseline':self.base,'parameters':vars(self.args),'provider_config_sha256':digest(config.read_bytes()),'model_policy':provider['model_policy'],'harness_sha256':digest(Path(__file__).read_bytes()),'manual_setup':'Public assignment and, in recovery, a labelled faulty producer. No post-launch operator retry, edit or acceptance.'}
  (self.out/'frozen.json').write_text(json.dumps(frozen,indent=2))
 def seed(self):
  seed=self.root/'fault-producer.py'
  seed.write_text('''#!'''+sys.executable+'''
import json,pathlib,sys
if '--version' in sys.argv:print('labelled deterministic fault producer');sys.exit(0)
prompt=sys.stdin.read();decoder=json.JSONDecoder();manifest=None
for i,c in enumerate(prompt):
 if c=='{':
  try:
   v,_=decoder.raw_decode(prompt[i:])
   if isinstance(v,dict) and set(('task','attempt','contract','outcome','criteria')).issubset(v):manifest=v
  except json.JSONDecodeError:pass
assert manifest and manifest['task']=='recover-number'
pathlib.Path('number.py').write_text('def answer():\\n    return 0\\n')
pathlib.Path('docs').mkdir(exist_ok=True)
pathlib.Path('docs/recover-number.md').write_text('Controlled faulty producer fixture. number.answer returns 0 instead of 42. The positive delivery claim below is intentionally untrusted negative-test input; it is not a successful test or acceptance. The real independent reviewer must reject the actual source.\\n')
manifest['outcome']='complete'
for c in manifest['criteria']:
 c['status']='pass';c['reason']='Deliberately wrong producer claim in the labelled negative fixture; actual source remains 0 and the reviewer must detect it.';c['evidence']=['number.py']
pathlib.Path('docs/recover-number.delivery.json').write_text(json.dumps(manifest))
print('Labelled fault fixture completed; no model invoked and no engine verdict supplied.')
''');seed.chmod(0o700)
  self.config['providers']['fault-fixture']={'command':str(seed),'args':[],'env_allow':[]};(self.root/'.swarm/providers.json').write_text(json.dumps(self.config))
  self.work=self.show()['work'];request=dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=self.work['revision'],task_id='recover-number',provider='fault-fixture',role='worker',workspace=str(self.project),instruction='Create the labelled faulty candidate for the negative recipe, not a real model result.',capture_output=True,timeout_seconds=0)
  preflight=self.cli(['agent','preflight',self.work['id']],request);(self.out/'seed-preflight.json').write_text(json.dumps(preflight,indent=2))
  a=self.cli(['agent','start',self.work['id']],request)['agent'];deadline=time.monotonic()+self.args.cli_seconds
  while time.monotonic()<deadline:
   current=self.cli(['agent','show',a['id']])['agent']
   if current['status'] not in ('queued','starting','running','stopping'):break
   time.sleep(self.args.poll_seconds)
  else:raise RuntimeError('Fault fixture did not end within observation period; original state retained')
  if current['status']!='completed':raise RuntimeError('Fault fixture failed; see public agent receipt')
  # These are setup operations BEFORE autonomous recovery. They do not edit
  # the result/receipt and never issue retry, review or acceptance themselves.
  self.cli(['profile',self.work['id'],'recover-number'],self.profile);self.cli(['profile',self.work['id']],self.profile)
  self.show();(self.out/'fault-injection.json').write_text(json.dumps({'type':'deterministic producer double','agent':a['id'],'attempt':a['attempt_id'],'no_model_call':True,'original_positive_claim':'deliberately untrusted test input, never a control/review/acceptance verdict','setup_finished':utc()},indent=2))
 def execute(self):
  result={'status':'FAIL','work':self.work['id'],'started':utc(),'operator_interventions_after_start':0};started=time.monotonic();last=None;review_refusals={};max_workers=0
  try:
   self.cli(['mission','preview',self.work['id']],self.profile);self.cli(['mission','start',self.work['id']],self.profile)
   watchlog=(self.out/'watch-private.log').open('ab');self.watch=subprocess.Popen([str(self.binary),'--root',str(self.root),'mission','watch',self.work['id']],stdout=watchlog,stderr=watchlog)
   while time.monotonic()-started<self.args.observation_seconds:
    observed=self.show();p=self.work['planning'];agents=self.cli(['agent','list',self.work['id']])['agents'];active=[x['agent'] for x in agents if x['agent']['status'] in ('queued','starting','running','stopping')];max_workers=max(max_workers,len(active))
    for task in self.work['tasks']:
     review=task.get('independent_review')
     if review and review['state']=='changes_requested':review_refusals[review['id']]=review
    state=(self.work['revision'],tuple((t['id'],t['status']) for t in self.work['tasks']),len(active))
    if state!=last:print(json.dumps({'seconds':round(time.monotonic()-started,1),'revision':state[0],'tasks':state[1],'active_workers':state[2],'planning_activations':p['activations'],'review_calls':p['reviewer']['calls']},ensure_ascii=False),flush=True);last=state
    if p.get('failure') or p['reviewer'].get('failure'):raise RuntimeError('Native planning/review failure; see immutable snapshots')
    if p['judge']['closed']:break
    if self.watch.poll() is not None:raise RuntimeError('Conductor ended before final judgment; original state retained')
    time.sleep(self.args.poll_seconds)
   else:raise RuntimeError('Operator observation period ended; this is not a business completion verdict')
   assert p['judge']['verdict']=='end' and all(t['status']=='accepted' for t in self.work['tasks'])
   assert observed['validation']['validated']==self.args.tasks and observed['validation']['stale']==0
   if self.args.scenario=='recovery':
    task=self.work['tasks'][0];assert len(task['attempts'])==2,'Corrective attempt not observed';assert review_refusals,'No actual semantic refusal observed'
    assert any(r['state']=='changes_requested' for r in task.get('previous_reviews',[])),'Original refusal not preserved'
    decisions=[e['payload'] for e in observed['events'] if e['kind']=='planning.decide'];assert any(set(d.get('input_events',[])) & set(review_refusals) and any(o.get('kind')=='retry' and o.get('id')==task['id'] and o.get('next') for o in d.get('operations',[])) for d in decisions),'Attributed supervisor retry decision missing'
   bundle=self.out/'delivery.bundle';self.cli(['planning','bundle',self.work['id'],str(bundle)]);delivery=self.out/'delivery';subprocess.run(['git','clone',str(bundle),str(delivery)],capture_output=True,check=True)
   exact='import number;assert number.answer()==42' if self.args.scenario=='recovery' else ';'.join(f'import module_{i+1};assert module_{i+1}.answer()=={i+1}' for i in range(self.args.tasks))
   verified=subprocess.run([sys.executable,'-B','-c',exact],cwd=delivery,capture_output=True,text=True);assert verified.returncode==0;assert self.git('rev-parse','HEAD')==self.base and self.git('status','--porcelain')==''
   spending=self.cli(['mission','spending',self.work['id']]);(self.out/'spending.json').write_text(json.dumps(spending,indent=2));(self.out/'final-public-work.json').write_text(json.dumps(observed,ensure_ascii=False,indent=2));(self.out/'agents.json').write_text(json.dumps(agents,indent=2))
   result.update(status='PASS',candidate=p['repository']['candidate_commit'],root_judge=p['judge'],max_simultaneous_workers=max_workers,review_refusals=list(review_refusals.values()),tasks=self.args.tasks,planning_calls=p['activations'],review_calls=p['reviewer']['calls'],bundle_sha256=digest(bundle.read_bytes()),functional_check={'command':[sys.executable,'-B','-c',exact],'exit':verified.returncode},scope='Controlled faulty initial producer; subsequent decisions/reviews/repair are real Codex' if self.args.scenario=='recovery' else 'Real Codex campaign at explicit concurrency')
  except Exception as error:result['error']=str(error) or type(error).__name__
  finally:
   self.cli(['mission','pause',self.work['id']])
   if self.watch and self.watch.poll() is None:self.watch.send_signal(signal.SIGINT);self.watch.wait(timeout=self.args.cli_seconds)
   for x in self.cli(['agent','list',self.work['id']])['agents']:
    if x['agent']['status'] in ('queued','starting','running','stopping'):self.cli(['agent','stop',x['agent']['id']])
   result.update(finished=utc(),elapsed_seconds=time.monotonic()-started);(self.out/'result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps(result,ensure_ascii=False),flush=True)
  return 0 if result['status']=='PASS' else 1

def main():
 p=argparse.ArgumentParser(description=__doc__)
 for name in ['binary','output','providers','provider','scenario']:p.add_argument('--'+name,required=True,choices=['recovery','parallel'] if name=='scenario' else None)
 for name in ['tasks','slots','activations','decisions','reviews','observation-seconds','cli-seconds','control-seconds']:p.add_argument('--'+name,type=int,required=True)
 p.add_argument('--poll-seconds',type=float,required=True);mode=p.add_mutually_exclusive_group();mode.add_argument('--setup-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');mode.add_argument('--resume-mission',action='store_true');p.add_argument('--resume-setup',action='store_true');p.add_argument('--control-program',required=True,choices=['python','python3'])
 a=p.parse_args()
 if a.resume_setup and not a.setup_only:p.error('--resume-setup requires --setup-only; never replay paid execution as setup')
 if any(getattr(a,k)<=0 for k in ['tasks','slots','activations','decisions','reviews','observation_seconds','cli_seconds','control_seconds','poll_seconds']):p.error('Positive explicit campaign inputs required')
 if a.scenario=='recovery' and (a.tasks!=1 or a.slots!=1):p.error('The bounded recovery recipe uses one task and one slot')
 campaign=Campaign(a)
 if a.run_prepared or a.resume_mission:
  frozen=json.loads((campaign.out/'frozen.json').read_text())
  excluded={'setup_only','resume_setup','run_prepared','resume_mission'}
  assert all(vars(a)[k]==v for k,v in frozen['parameters'].items() if k not in excluded),'Prepared campaign parameters changed'
  assert digest(campaign.binary.read_bytes())==frozen['binary_sha256'],'Prepared binary changed'
  if a.run_prepared:assert digest((campaign.root/'.swarm/providers.json').read_bytes())==frozen['provider_config_sha256'],'Prepared provider configuration changed'
  else:
   current=json.loads((campaign.root/'.swarm/providers.json').read_text());source=json.loads(Path(a.providers).read_text());assert current['providers'][a.provider]==source['providers'][a.provider] and current['providers'][a.provider]['model_policy']==frozen['model_policy'],'Selected provider changed; use a public configuration replan'
  campaign.work=campaign.cli(['planning','show',frozen['work']]);campaign.base=frozen['baseline'];campaign.config=json.loads((campaign.root/'.swarm/providers.json').read_text());campaign.profile=dict(provider=a.provider,role='worker',workspace=str(campaign.project),capture_output=True,timeout_seconds=0)
  if a.run_prepared:assert all(not t.get('attempts') for t in campaign.work['tasks']),'Prepared work has attempts: preserve and replan instead of reseeding'
  elif a.scenario=='recovery' and not (campaign.out/'fault-injection.json').exists():
   agents=[x['agent'] for x in campaign.cli(['agent','list',campaign.work['id']])['agents']];faults=[x for x in agents if x['provider']=='fault-fixture' and x['status']=='completed'];assert len(faults)==1,'Original fault producer not attributable'
   (campaign.out/'fault-injection.json').write_text(json.dumps({'type':'deterministic producer double','agent':faults[0]['id'],'attempt':faults[0]['attempt_id'],'no_model_call':True,'observed_after_setup_metadata_error':True,'original_positive_claim':'deliberately untrusted negative input, never engine acceptance','observed_at':utc()},indent=2))
  (campaign.out/'execution-inputs.json').write_text(json.dumps({'at':utc(),'parameters':vars(a),'harness_sha256':digest(Path(__file__).read_bytes()),'prepared_sha256':digest((campaign.out/'frozen.json').read_bytes())},indent=2))
 else:campaign.configure()
 if a.setup_only:print(json.dumps({'status':'SETUP_ONLY','paid_calls':0,'work':campaign.work['id'],'root':str(campaign.root)}));return 0
 if a.scenario=='recovery' and not a.resume_mission:campaign.seed()
 return campaign.execute()

if __name__=='__main__':raise SystemExit(main())
