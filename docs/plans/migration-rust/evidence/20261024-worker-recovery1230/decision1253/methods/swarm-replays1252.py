import os,subprocess,json,time,concurrent.futures
from pathlib import Path
base=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230/recipes1252');base.mkdir(mode=0o700)
env={**os.environ,'SWARM_STATUS_CONTRACT_BINARY':str(Path('.cache/rust-migration-binaries/swarm-rust-7292e53a8fc2c24a7c9b343bad79f7bfe0cd6ef6989bef6340e5c70baec0ca6b').resolve()),'SWARM_STATUS_CONTRACT_PROFILE':str(Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230/normal1249/result.json').resolve())}
def run(name):
 cmd=['python3','tools/verification/recipe.py','run','--recipe',f'tools/verification/recipes/{name}.json','--output',str(base/name)]
 with (base/(name+'.stdout')).open('x') as out,(base/(name+'.stderr')).open('x') as err:p=subprocess.run(cmd,env=env,stdout=out,stderr=err)
 print(name,p.returncode,flush=True)
 if p.returncode==0:
  verified=subprocess.run(['python3','tools/verification/recipe.py','verify','--evidence',str(base/name)],capture_output=True)
  (base/(name+'.verify.stdout')).write_bytes(verified.stdout);(base/(name+'.verify.stderr')).write_bytes(verified.stderr)
  if verified.returncode:raise RuntimeError(name+' integrity failure')
 else:raise RuntimeError(name+' original retained; diagnose before any retry')
 return name
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 for result in pool.map(run,['mission-worker-completion-recovery','mission-watch-registered-observer','native-http-mission']):pass
