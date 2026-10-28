#!/usr/bin/env python3
"""Real served lifecycle inventory on an owned Root with unapplied synthetic rows."""
import argparse,json,subprocess,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server
from rust_agent_exchanges import logical
from rust_lifecycle_inventory import seed

def run(binary,output):
    """Exercise UI without providers or business writes; compare all logical SQL."""
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'lifecycle-browser');seed(f);before=logical(f.root);s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None
    try:
        payload=dict(address=s.address,key=s.key,root=str(f.root),work=f.work,output=str(output/'browser'))
        with (output/'browser.stdout').open('xb') as out,(output/'browser.stderr').open('xb') as err:r=subprocess.run(['node','tests/rust_lifecycle_inventory_ui.cjs'],input=json.dumps(payload).encode(),stdout=out,stderr=err)
        assert r.returncode==0,('Browser failed',r.returncode);proof=json.loads((output/'browser/result.json').read_bytes());assert proof['status']=='PASS' and len(proof['checks'])==4;assert logical(f.root)==before and not list(f.root.rglob('model-received.json'))
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as error:
        if primary:raise BaseExceptionGroup('History browser and owned cleanup',[primary,error])
        raise
    if primary:raise primary
    assert not s.capture_errors
    for p in output.rglob('*'):
        if p.is_file():assert s.key.encode() not in p.read_bytes()
    publish(output/'result.json',dict(status='PASS',checks=4,binary_sha256=digest(binary.read_bytes()),provider_requested=False,live_root_mutation=False,synthetic_metadata_not_applied=True));print('LIFECYCLE_INVENTORY_BROWSER_VERIFIED 4',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
