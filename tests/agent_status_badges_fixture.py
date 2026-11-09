"""Prepare an isolated browser fixture from exact qualified compiler assets.

No server, provider or agent is started here. Inputs are a local delivery record
and fresh evidence directory. Output is an owned governed fixture with retained
asset hashes; HTML/control code and recorded-state doubles are explicitly tests.
Run a loopback static server separately and use CUA. Browser checks are not native
mission observations or independent acceptance. Existing outputs are not replaced.
"""
from pathlib import Path
import argparse,json,tarfile,hashlib,tempfile,shutil
from fixture_area import fixture_area
p=argparse.ArgumentParser();p.add_argument('--record',required=True,type=Path);p.add_argument('--evidence',required=True,type=Path);a=p.parse_args();record=json.loads(a.record.read_text());profile=Path(record['evidence'])/'build/profile';result=json.loads((profile/'result.json').read_text());assert result['no_fixture_feature'] and result['sha256']==record['binary_sha256']
root=Path(tempfile.mkdtemp(prefix='agent-badge-browser-',dir=fixture_area()));assets=['swarm-logo.png','wattson_themes.css','cockpit.css','cockpit-ux.css','swarm-design.css','swarm-workbench.css','i18n-en.js','i18n.js','swarm-workbench.js','cockpit-native-panels.js'];hashes={}
with tarfile.open(profile/'effective-sources.tar.gz') as tar:
 for name in assets:
  matches=[m for m in tar.getmembers() if m.name.endswith('web/'+name)];assert len(matches)==1;data=tar.extractfile(matches[0]).read();hashes[name]=hashlib.sha256(data).hexdigest();assert hashes[name]==record['effective_inputs']['web/'+name];(root/name).write_bytes(data)
for name in ['index.html','fixture.js']:shutil.copyfile(Path(__file__).parent/'fixtures/agent-status-badges'/name,root/name)
v=dict(version=record['version'],binary_sha256=record['binary_sha256'],root=str(root),assets=hashes,scope='Explicit inert metadata doubles; exact compiler assets; no live agent claims');(root/'qualification.json').write_text(json.dumps(v,indent=2)+'\n');a.evidence.mkdir(parents=True,exist_ok=True);out=a.evidence/'browser-fixture.json';assert not out.exists();out.write_text(json.dumps(v,indent=2)+'\n');print(root)
