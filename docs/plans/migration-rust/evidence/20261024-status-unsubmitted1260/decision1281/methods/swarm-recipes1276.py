from pathlib import Path
import subprocess,os,json,concurrent.futures
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260/recipes1276');b.mkdir(mode=0o700)
env={**os.environ,'SWARM_STATUS_CONTRACT_BINARY':str(Path('.cache/rust-migration-binaries/swarm-rust-a190e3550e464c1fdcdb28c1d2fbd886cd2dfdc70b7904f90e6169c557a8d484').resolve()),'SWARM_STATUS_CONTRACT_PROFILE':str(Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260/normal1273/result.json').resolve())}
def run(name):
 cmd=['python3','tools/verification/recipe.py','run','--recipe',f'tools/verification/recipes/{name}.json','--output',str(b/name)]
 with (b/(name+'.stdout')).open('x') as out,(b/(name+'.stderr')).open('x') as err:p=subprocess.run(cmd,env=env,stdout=out,stderr=err)
 print(name,p.returncode,flush=True)
 if p.returncode:raise RuntimeError(name+' original failure retained; diagnosis required')
 proof=subprocess.run(['python3','tools/verification/recipe.py','verify','--evidence',str(b/name)],capture_output=True)
 (b/(name+'.verify.stdout')).write_bytes(proof.stdout);(b/(name+'.verify.stderr')).write_bytes(proof.stderr)
 assert proof.returncode==0
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(run,['mission-worker-completion-recovery','mission-status-review']))
