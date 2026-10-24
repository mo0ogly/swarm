from pathlib import Path
import concurrent.futures,subprocess,json,time,sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-two-task1300').resolve()
roles=json.loads((b/'campaign.json').read_text())
def call(role):
 d=b/role;start=time.time()
 cmd=['python3','/home/fpizzi/.codex/skills/skynet-agents/scripts/call.py','--api-base','https://chat.skynet.cossi.internet/openai','--model','glm-5.2','--key-env','SKYNET_CHAT_API_KEY','--prompt',str(d/'prompt.txt'),'--context',str(d/'context.txt'),'--max-output-tokens','2000','--reasoning-effort','low','--out',str(d/'response')]
 with (d/'launcher.stdout').open('x') as out,(d/'launcher.stderr').open('x') as err:
  p=subprocess.run(cmd,stdout=out,stderr=err)
 result={'role':role,'exit':p.returncode,'seconds':round(time.time()-start,3)}
 atomic(d/'launcher.json',json.dumps(result,indent=2)+'\n')
 print(json.dumps(result),flush=True);return result
print('START eight GLM roles, two simultaneous HTTP calls; no retries',flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
 results=list(pool.map(call,roles))
atomic(b/'launch-results.json',json.dumps(results,indent=2)+'\n')
print('ALL EIGHT FINISHED',flush=True)
