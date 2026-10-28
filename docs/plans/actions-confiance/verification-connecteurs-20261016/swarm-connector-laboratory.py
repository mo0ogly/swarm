import datetime,hashlib,json,os,re,subprocess,sys
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=repo/'docs/plans/actions-confiance/verification-connecteurs-20261016'
def stamp():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
if len(sys.argv)==1:
 log=p/'connector-laboratory-runner.out'
 with log.open('wb') as f:child=subprocess.Popen([sys.executable,__file__,'run'],cwd=repo,stdout=f,stderr=f,start_new_session=True)
 proc=Path('/proc')/str(child.pid)
 v={'pid':child.pid,'proc_start_ticks':(proc/'stat').read_text().split(') ',1)[1].split()[19],'argv':[sys.executable,__file__,'run'],'at':stamp()}
 (p/'connector-laboratory-runner.json').write_text(json.dumps(v,indent=2)+'\n');print(json.dumps(v));sys.exit(0)
files=sorted((repo/'internal/engine').glob('*.go'))+sorted(repo.glob('*.go'))+sorted(f for f in (repo/'web').rglob('*') if f.is_file())+[repo/'go.mod',repo/'go.sum'];inputs={str(f.relative_to(repo)):sha(f) for f in files}
stages=[('connector-regression-laboratory',r'^TestActionsConfianceT6Laboratory')]
for name,pattern in stages:
 argv=['go','test','-race','-count=1','-timeout','4m','-run',pattern,'-v','./internal/engine'];start=stamp()
 with (p/(name+'.out')).open('wb') as f:r=subprocess.run(argv,cwd=repo,stdout=f,stderr=f,timeout=600)
 raw=(p/(name+'.out')).read_text();passed=re.findall(r'^\s*--- PASS: (\S+)',raw,re.M)
 v={'argv':argv,'started_at':start,'ended_at':stamp(),'exit_code':r.returncode,'named_passed_cases':passed,'input_sha256':inputs,'sources_unchanged':all(sha(repo/f)==h for f,h in inputs.items()),'output_sha256':sha(p/(name+'.out'))}
 (p/(name+'.json')).write_text(json.dumps(v,indent=2)+'\n')
 print(json.dumps({'stage':name,'exit_code':r.returncode,'passed_cases':len(passed)}),flush=True)
 if r.returncode or not passed or not v['sources_unchanged']:sys.exit(1)
