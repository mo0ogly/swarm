#!/usr/bin/env python3
"""Run real served activity modal on an owned inert Root, never a provider/live mission."""
import argparse,json,os,subprocess,traceback,sqlite3
from pathlib import Path
import rust_agent_model_route as retained
from rust_native_http_cli import Server

def run(binary,output):
    """Preserve original browser/process evidence and all owned cleanup failures.
    No global deadline or hidden retry. Session key only travels over child stdin.
    """
    output.mkdir(parents=True,exist_ok=False);fixture=retained.Fixture(binary,output,'web-policy-browser');
    with sqlite3.connect(fixture.root/'.swarm/state.db') as c:c.execute('INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)',(fixture.work,'9999-01-01T00:00:00Z','dispatch','<img src=x onerror=window.activityInjected=true>'))
    before=fixture.snapshot();view=fixture.ok(['run-limits','activity','show']);assert view['schema_version']==1 and view['values']['page_default_rows'];server=Server(binary,fixture.root,output,'http',environment=fixture.environment);server.capture_async(output/'stream.jsonl');primary=None
    try:
        payload={'binary':str(binary),'root':str(fixture.root),'address':server.address,'key':server.key,'output':str(output/'browser'),'work':fixture.work,'initial_entries':len(fixture.ok(['activity','show',fixture.work])['entries'])}
        with (output/'browser.stdout').open('xb') as stdout,(output/'browser.stderr').open('xb') as stderr:
            result=subprocess.run(['node','tests/rust_activity_ui.cjs'],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
        assert result.returncode==0,('Browser failed',result.returncode)
        proof=json.loads((output/'browser/result.json').read_bytes());assert proof['status']=='PASS' and len(proof['checks'])==4
        assert fixture.snapshot()==before and not list(fixture.root.rglob('model-received.json'))
    except BaseException as error:primary=error
    try:server.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Browser and cleanup failures',[primary,cleanup])
        raise
    if primary:raise primary
    assert not server.capture_errors
    for path in output.rglob('*'):
        if path.is_file():assert server.key.encode() not in path.read_bytes()
    retained.publish(output/'result.json',{'status':'PASS','checks':4,'binary_sha256':retained.digest(binary.read_bytes()),'provider_scope':'no provider contacted'});print('ACTIVITY_BROWSER_VERIFIED 4',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
