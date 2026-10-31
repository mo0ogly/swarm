"""Trace only the second owned inert worker, no external process or account.
The first failure and task remain intact; a distinct todo task permits observing
the failing syscall rather than retrying the completed attempt unchanged.
"""
import os,sys,json,subprocess
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_agent_model_route as route
base=Path(__file__).resolve().parent;out=base/'managed-trace';out.mkdir(exist_ok=False)
f=route.Fixture.__new__(route.Fixture);f.output=out;f.label='trace';f.calls=[]
f.binary=Path(json.loads((base/'candidate-first/result.json').read_text())['binary'])
f.root=Path(json.loads((base/'managed-positive-setup/fixture.json').read_text())['root'])
f.work=json.loads((base/'managed-positive-setup/result.json').read_text())['planned_work']
f.environment={**os.environ,'CODEX_HOME':str(f.root/'codex-home')}
w=f.ok(['work','show',f.work])['work']
f.request=dict(schema_version=1,event_id='biba-trace',expected_revision=w['revision'],task_id='task-2',provider='fixture',capture_output=True)
f.prepare()
argv=['strace','-f','-e','trace=openat,ioctl,execve','-o',str(out/'syscalls.out'),str(f.binary),'agent','run','biba-trace','--root',str(f.root),'--json']
route.publish(out/'argv.json',dict(argv=argv,scope='Owned fixture process descendants only'))
r=subprocess.run(argv,env=f.environment,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
route.publish(out/'stdout',r.stdout);route.publish(out/'stderr',r.stderr)
print('CLI exit',r.returncode)
