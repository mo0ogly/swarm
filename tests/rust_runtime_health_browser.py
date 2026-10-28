#!/usr/bin/env python3
"""Real served storage diagnostic/Admin path, owned Root without queued Agent."""
import argparse,json,subprocess,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server
from rust_agent_exchanges import logical

def run(binary,output):
    """Keep observed UI/process evidence and all cleanup causes, no watchdog.
    Initial SQL state is unchanged; intended saved Admin thresholds persist only
    in this owned fixture. Session key travels over child stdin, never reports.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'health-browser');before=logical(f.root);initial=f.ok(['runtime-health']);s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None
    try:
        payload=dict(address=s.address,key=s.key,root=str(f.root),output=str(output/'browser'),initial_state=initial['state'])
        with (output/'browser.stdout').open('xb') as stdout,(output/'browser.stderr').open('xb') as stderr:r=subprocess.run(['node','tests/rust_runtime_health_ui.cjs'],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,('Browser failed',r.returncode)
        proof=json.loads((output/'browser/result.json').read_bytes());assert proof['status']=='PASS' and len(proof['checks'])==4
        values=f.ok(['engine-settings','show'])['values'];assert values['runtime_storage_stop_bytes']=='0' and values['runtime_storage_warning_bytes']==str((1<<63)-1) and values['runtime_storage_stop_inodes']=='0'
        assert f.ok(['runtime-health'])['state']=='warning' and logical(f.root)==before and not list(f.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    try:s.cleanup()
    except BaseException as e:
        if primary:raise BaseExceptionGroup('Browser and owned cleanup',[primary,e])
        raise
    if primary:raise primary
    assert not s.capture_errors
    for p in output.rglob('*'):
        if p.is_file():assert s.key.encode() not in p.read_bytes()
    publish(output/'result.json',dict(status='PASS',checks=4,binary_sha256=digest(binary.read_bytes()),provider_started=False,live_database=False));print('RUNTIME_HEALTH_BROWSER_VERIFIED 4',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
