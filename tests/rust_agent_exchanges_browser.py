#!/usr/bin/env python3
"""Real historical-exchange cockpit on owned fixture; no native send/ACK/provider."""
from pathlib import Path
import argparse,json,subprocess,traceback
from rust_agent_model_route import publish,digest
from rust_agent_exchanges import historical_fixture,logical
from rust_native_http_cli import Server

def run(binary,output):
    """Bind real prepared Work/Agent fixture and require public read capability.

    Original browser evidence and cleanup errors retained; no silent timeout or
    missing-control wait on an older binary. Entire logical DB must be unchanged.
    """
    output.mkdir(parents=True,exist_ok=False);f,other=historical_fixture(binary,output);rows=f.ok(['exchange','list',f.work]);assert len(rows)==2;before=logical(f.root);state=f.snapshot();server=Server(binary,f.root,output,'http',environment=f.environment);server.capture_async(output/'stream.jsonl');primary=None
    try:
        cfg=dict(binary=str(binary),root=str(f.root),work=f.work,other=other,address=server.address,key=server.key,output=str(output/'browser'))
        with (output/'browser.stdout').open('xb') as stdout,(output/'browser.stderr').open('xb') as stderr:r=subprocess.run(['node','tests/rust_agent_exchanges_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,('Browser failed',r.returncode);proof=json.loads((output/'browser/result.json').read_bytes());assert proof['status']=='PASS' and len(proof['checks'])==4;assert logical(f.root)==before and f.snapshot()==state and not list(f.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    try:server.cleanup()
    except BaseException as e:
        if primary:raise BaseExceptionGroup('Browser and cleanup',[primary,e])
        raise
    if primary:raise primary
    assert not server.capture_errors
    for path in output.rglob('*'):
        if path.is_file():assert server.key.encode() not in path.read_bytes()
    publish(output/'result.json',dict(status='PASS',checks=4,native_sha256=digest(binary.read_bytes()),provider_started=False,exchange_send=False,live_database=False));print('AGENT_EXCHANGES_BROWSER_VERIFIED 4',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
