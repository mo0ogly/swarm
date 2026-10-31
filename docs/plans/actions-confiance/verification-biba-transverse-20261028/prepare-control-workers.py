"""Prepare owned inert retained managed producers, no real model or acceptance.
Use the explicit historical normal profile for its existing socket-based fixture.
Only a future candidate executes the controls under test; no live mission touched.
"""
import sys
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_managed_auto_worker as managed
base=Path(__file__).resolve().parent
script="""from pathlib import Path
import os,json,hashlib
assert Path('tracked.txt').read_bytes()==b'Committed planning input\\n'
root=next(p.parent for p in Path.cwd().parents if p.name=='.swarm')
facts={}
try:
 with (root/'.swarm/engine-settings.json').open('rb') as f:facts['private_read']=bool(f.read(1))
except OSError:facts['private_read']=False
try:
 fd=os.open(root/'.swarm/state.db',os.O_RDWR);os.close(fd);facts['database_write']=True
except OSError:facts['database_write']=False
try:
 with (root/'.swarm'/('biba-control-'+str(os.getpid()))).open('xb') as f:f.write(b'OWNED_SYNTHETIC_LOW_CONTROL')
 facts['private_write']=True
except OSError:facts['private_write']=False
try:
 fd=os.open(root/'source/tracked.txt',os.O_WRONLY);os.close(fd);facts['official_write']=True
except OSError:facts['official_write']=False
with Path('biba-control-result-'+str(os.getpid())+'.json').open('x') as f:json.dump(facts,f)
print('OWNED_BIBA_CONTROL '+json.dumps(facts),flush=True)
"""
controls=[dict(id='biba-control',command=['python3','-c',script],criteria=[1],inputs=['tracked.txt'],justification='Observe only owned synthetic Biba boundaries; never accept a task',review_output=True)]
managed.worker.subreaper_scope(lambda:managed.run(base/'baseline-image',Path('.cache/local-delivery/rust-j9sqxnrn/build/profile/result.json').resolve(),base/'control-workers-before',controls=controls),lambda error:None,managed.parent_death.subreaper)
