#!/usr/bin/env python3
"""Recheck earlier policy HEAD wire bodies after client-suppression discovery.

Owned inert Root, valid GET before each HEAD, no full suites/provider/live state.
"""
from pathlib import Path
import argparse,http.client,traceback
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server,assert_head_wire

def run(binary,output):
    """Verify raw EOF/MIME/status, retain safe byte witnesses and Root immutability.

    Actual authenticated reads only. The witness avoids HTTPResponse's automatic
    suppression; TCP65536 is a read buffer, never a response cap or timeout.
    """
    output.mkdir(parents=True,exist_ok=False);f=Fixture(binary,output,'head-wire');before=f.snapshot();s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None;checks=[]
    try:
        for path,mime in [('/api/v1/web-requests','application/json'),('/api/v1/web-requests/script.js','application/javascript'),('/api/v1/graph-performance','application/json'),('/api/v1/run-limits?scope=project&mission_id=&scope_key=','application/json')]:
            c=http.client.HTTPConnection(s.host,s.port,timeout=None)
            try:c.request('GET',path,headers={'Cookie':s.cookie});r=c.getresponse();raw=r.read();assert r.status==200 and raw and r.getheader('Content-Type','').startswith(mime)
            finally:c.close()
            proof=assert_head_wire(s.host,s.port,path,s.cookie,content_type=mime);checks.append(dict(path=path,valid_get_control=True,**proof));publish(output/'checks.json',checks)
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    try:s.cleanup()
    except BaseException as e:
        if primary:raise BaseExceptionGroup('HEAD and cleanup',[primary,e])
        raise
    if primary:raise primary
    assert len(checks)==4 and not s.capture_errors;publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),provider_started=False,live_database=False));print('HTTP_HEAD_WIRE_PREVIOUS_SLICES_VERIFIED 4',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
