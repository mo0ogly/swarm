import datetime,hashlib,json,subprocess,sys
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=repo/'docs/plans/actions-confiance/verification-connecteurs-20261016'
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def stamp():return datetime.datetime.now(datetime.timezone.utc).isoformat()
if len(sys.argv)==1:
 with (p/'connector-labelled-build-runner.out').open('wb') as f:c=subprocess.Popen([sys.executable,__file__,'run'],cwd=repo,stdout=f,stderr=f,start_new_session=True)
 print(json.dumps({'pid':c.pid,'at':stamp()}));sys.exit(0)
files=list(repo.glob('*.go'))+list((repo/'internal/engine').glob('*.go'))+list((repo/'web').glob('*'))
inputs={str(f.relative_to(repo)):sha(f) for f in files if f.is_file()}
for name,argv,env in [('build',['sh','build.sh','bin/swarm-actions-confiance-connectors-labelled'],None),('vet',['env','CGO_ENABLED=0','go','vet','./internal/engine'],None),('i18n',['npm','run','test:i18n'],None),('contexts',['python3','tools/agent-workflows/check.py'],None),('diff',['git','diff','--check'],None)]:
 start=stamp()
 with (p/('connector-labelled-'+name+'.out')).open('wb') as f:r=subprocess.run(argv,cwd=repo,stdout=f,stderr=f,timeout=600)
 v={'argv':argv,'started_at':start,'ended_at':stamp(),'exit_code':r.returncode,'sources_unchanged':all(sha(repo/f)==h for f,h in inputs.items()),'output_sha256':sha(p/('connector-labelled-'+name+'.out')),'input_sha256':inputs}
 if name=='build' and not r.returncode:v['binary_sha256']=sha(repo/'bin/swarm-actions-confiance-connectors-labelled')
 (p/('connector-labelled-'+name+'.json')).write_text(json.dumps(v,indent=2)+'\n');print(json.dumps({'stage':name,'exit_code':r.returncode}),flush=True)
 if r.returncode or not v['sources_unchanged']:sys.exit(1)
