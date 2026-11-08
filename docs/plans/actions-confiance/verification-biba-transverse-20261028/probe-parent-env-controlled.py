"""Read only a synthetic owned parent's marker, never a foreign environment."""
import os,sys,json,tempfile,subprocess
from pathlib import Path
base=Path(__file__).resolve().parent
lines=(base/'native-permanent-socketpair/step-0001.stdout').read_text().splitlines()
driver=Path(next(v.split(' ',1)[1] for v in lines if v.startswith('BIBA_BOUNDARY_EVIDENCE ')))/'boundary-driver'
root=Path(tempfile.mkdtemp(prefix='swarm-biba-parent-env-'));(root/'.swarm').mkdir();workspace=root/'out';workspace.mkdir()
script=root/'provider.py';script.write_text("import json\ntry:\n data=open('/proc/"+str(os.getpid())+"/environ','rb').read();found=b'BIBA_OWNED_PARENT_MARKER=OWNED_PARENT_ONLY' in data\nexcept OSError as error:found=False\nprint(json.dumps({'owned_parent_marker_read':found}))\n")
valid=subprocess.run(['/usr/bin/python3',str(script)],env={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8'},capture_output=True)
assert valid.returncode==0 and json.loads(valid.stdout)['owned_parent_marker_read'] is True
result=subprocess.run([str(driver),str(root),str(workspace),str(script)],env={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8'},capture_output=True)
assert result.returncode==0 and json.loads(result.stdout)['owned_parent_marker_read'] is False
v=dict(valid_control=True,root=str(root),parent_pid=os.getpid(),parent_environment_scope='Only synthetic marker, PATH and LANG supplied by launcher',exit=result.returncode,stdout=result.stdout.decode(),stderr=result.stderr.decode())
(base/'parent-env-controlled.json').write_text(json.dumps(v,indent=2)+'\n');print(json.dumps(v))
