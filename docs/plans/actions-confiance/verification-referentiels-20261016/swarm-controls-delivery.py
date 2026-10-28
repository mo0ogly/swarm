import concurrent.futures,datetime,hashlib,json,os,signal,subprocess,sys,time
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');proof=repo/'docs/plans/actions-confiance/verification-referentiels-20261016';binary=repo/'bin/swarm-actions-confiance-controls'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def stamp():return datetime.datetime.now(datetime.timezone.utc).isoformat()
if len(sys.argv)==1:
 with (proof/'controls-delivery-runner.out').open('wb')as out:child=subprocess.Popen([sys.executable,__file__,'run'],cwd=repo,stdout=out,stderr=out,start_new_session=True)
 receipt={'pid':child.pid,'argv':[sys.executable,__file__,'run'],'proc_start_ticks':(Path('/proc')/str(child.pid)/'stat').read_text().split(') ',1)[1].split()[19],'at':stamp()};(proof/'controls-delivery-runner.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));sys.exit(0)
deadline=time.monotonic()+300
while not (proof/'controls-current-build.json').exists():
 if time.monotonic()>=deadline:raise RuntimeError('build receipt unavailable; no test started')
 time.sleep(.1)
build=json.loads((proof/'controls-current-build.json').read_text());assert build['exit_code']==0 and build['sources_unchanged'] and sha(binary)==build['binary_sha256']
inputs={str(p.relative_to(repo)):sha(p)for p in [*repo.glob('*.go'),*(repo/'internal/engine').glob('*.go'),*(p for p in (repo/'web').rglob('*')if p.is_file()),repo/'tests/actions_confiance_connectors.py',repo/'tests/actions_confiance_connectors_ui.cjs',repo/'tools/actions-confiance/controls.py',repo/'tools/actions-confiance/test_controls.py',repo/'tests/actions_confiance_controls.py',repo/'tests/actions_confiance_controls_ui.cjs']}
stages=[('controls-public',['python3','tests/actions_confiance_controls.py','--root',str(repo),'--evidence-dir',str(proof/'controls-public')]),('controls-browser',['node','tests/actions_confiance_controls_ui.cjs',str(binary),str(proof/'controls-browser')]),('controls-connector-browser',['node','tests/actions_confiance_connectors_ui.cjs',str(binary),str(proof/'controls-connector-browser')]),('controls-pilot',['python3','tests/actions_confiance_t2.py','--criterion','0','--root',str(repo),'--timeout-seconds','120','--binary',str(binary),'--evidence-dir',str(proof/'controls-pilot')])]
def run(stage):
 name,argv=stage;start=stamp();fd=os.open(proof/(name+'.out'),os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600)
 with os.fdopen(fd,'wb')as out:
  child=subprocess.Popen(argv,cwd=repo,stdout=out,stderr=out,start_new_session=True)
  try:code=child.wait(timeout=240)
  except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait();code=124
 value={'argv':argv,'started_at':start,'ended_at':stamp(),'exit_code':code,'binary_sha256':sha(binary),'sources_unchanged':all(sha(repo/p)==h for p,h in inputs.items()),'input_sha256':inputs,'output_sha256':sha(proof/(name+'.out'))};(proof/(name+'.json')).write_text(json.dumps(value,indent=2)+'\n');return {'stage':name,'exit_code':code,'sources_unchanged':value['sources_unchanged']}
with concurrent.futures.ThreadPoolExecutor(max_workers=3)as pool:
 results=list(pool.map(run,stages))
for result in results:print(json.dumps(result),flush=True)
assert all(r['exit_code']==0 and r['sources_unchanged']for r in results)
