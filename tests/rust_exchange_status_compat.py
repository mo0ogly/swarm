#!/usr/bin/env python3
"""Recheck prior mission-status consumer of the shared exchange artifact reader.

Old/new default binaries, fresh inert Roots, source-type fault after valid control.
No provider/send/ACK/live DB; timestamp/status prose is not compared as identity.
"""
from pathlib import Path
import argparse,json,sqlite3,http.client,urllib.parse,traceback
from rust_agent_model_route import publish,digest
from rust_agent_exchanges import historical_fixture,logical
from rust_native_http_cli import Server,clean

def run(before_binary,binary,output):
    """Reproduce old TEXT refusal and assert new status accepts TEXT/Go UTF8.

    Both routes really traverse mission_status_inputs→shared exchanges. Require
    valid BLOB status first, preserve each response and logical nonmutation plus
    all owned cleanup causes. No arbitrary watchdog or exception by Work ID.
    """
    output.mkdir(parents=True,exist_ok=False);checks=[]
    for label,exe in [('before',before_binary),('after',binary)]:
        out=output/label;out.mkdir();f,other=historical_fixture(exe,out);s=Server(exe,f.root,out,'http',environment=f.environment);s.capture_async(out/'stream.jsonl');primary=None;responses=[]
        def status(expected):
            c=http.client.HTTPConnection(s.host,s.port,timeout=None)
            try:c.request('GET','/api/v1/mission?'+urllib.parse.urlencode({'action':'status','work':f.work}),headers={'Cookie':s.cookie});r=c.getresponse();raw=r.read();actual=r.status
            finally:c.close()
            value=json.loads(raw);responses.append(dict(status=actual,value=clean(value)));publish(out/'responses.json',responses);assert actual==expected,(actual,clean(value));return value
        try:
            before=logical(f.root);control=status(200);assert isinstance(control['tasks'],list);assert logical(f.root)==before;checks.append(label+'-valid-blob-status-control-no-mutation')
            with sqlite3.connect(f.root/'.swarm/state.db') as c:c.execute('UPDATE agent_exchanges SET artifacts=CAST(artifacts AS TEXT) WHERE id=?',('a-first',))
            text=logical(f.root);value=status(500 if label=='before' else 200);assert logical(f.root)==text
            if label=='after':
                assert value['tasks']==control['tasks']
                with sqlite3.connect(f.root/'.swarm/state.db') as c:c.execute('UPDATE agent_exchanges SET artifacts=? WHERE id=?',(b'[{"path":"invalid-\xff","sha256":"declared"}]','a-first'))
                utf8=logical(f.root);status(200);assert logical(f.root)==utf8
            assert not list(f.root.rglob('model-received.json'));checks.append(label+'-text-and-after-utf8-status-boundary')
        except BaseException as e:primary=e
        try:s.cleanup()
        except BaseException as e:
            if primary:raise BaseExceptionGroup('Status and cleanup',[primary,e])
            raise
        if primary:raise primary
        assert not s.capture_errors
    assert len(checks)==4;publish(output/'result.json',dict(status='PASS',checks=checks,before_sha256=digest(before_binary.read_bytes()),after_sha256=digest(binary.read_bytes()),provider_started=False,exchange_send=False,live_database=False));print('EXCHANGE_STATUS_COMPAT_VERIFIED 4',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--before-binary',required=True,type=Path);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.before_binary.resolve(),a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
