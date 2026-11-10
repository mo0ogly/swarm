#!/usr/bin/env python3
"""Historical exchanges: real Go/native CLI and protected HTTP, inert fixture only.

Fixture SQL creates retained rows after public Work/Agent preparation, never
claims native send/ACK support. No agent run, provider, live DB or global timeout.
"""
from pathlib import Path
import argparse,json,sqlite3,subprocess,http.client,urllib.parse,traceback
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server,clean,assert_head_wire

def historical_fixture(binary,output):
    """Prepare a local inert Agent, then insert attributable historical fixtures.

    Foreign-key references are valid. No provider is executed; the rows are test
    data rather than receipts from an exchange operation. Retain original DB
    logical state after population so every public read can prove no mutation.
    """
    f=Fixture(binary,output,'exchange-read');prepared=f.prepare();agent=prepared['agent']['id'];db=f.root/'.swarm/state.db'
    other=f.ok(['work','create'],dict(schema_version=1,event_id='other-work',expected_revision=0,title='Other exchange scope',objective='Read isolation',scope='Owned fixture',criteria=['No cross-work rows']))['work']['id']
    with sqlite3.connect(db) as c:
        c.execute('PRAGMA foreign_keys=ON')
        for identifier,artifacts in [('z-last',b'null'),('a-first',b'[null,{"path":"first","path":null,"sha256":"declared"}]')]:
            c.execute('INSERT INTO agent_exchanges(id,request_hash,work_id,kind,source_agent_id,source_task_id,source_attempt_id,recipient_task_id,recipient_role,state,need,artifacts,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)',(identifier,'fixture-unverified',f.work,'handoff',agent,'task','fixture-attempt','task','worker','pending','<img src=x onerror=window.exchangeInjected=true> & retained text',artifacts,'2026-01-01T00:00:00Z'))
    publish(output/'historical-fixture.json',dict(work=f.work,other_work=other,agent=agent,source='isolated SQL fixture after public preparation; no native exchange send/ACK',live_database=False))
    return f,other

def logical(root):
    """Snapshot full logical fixture SQL with storage types and raw TEXT bytes.

    Unlike iterdump(), invalid UTF-8 TEXT remains representable. Schema names
    are quoted, values never interpolated. One readonly transaction and checked
    rollback/close preserve causes; data/credentials are not emitted to logs.
    """
    c=sqlite3.connect((root/'.swarm/state.db').absolute().as_uri()+'?mode=ro',uri=True);c.text_factory=bytes;primary=None;result=None
    try:
        c.execute('BEGIN');schema=list(c.execute('SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name'));tables=[]
        quote=lambda n:'"'+n.decode('utf-8').replace('"','""')+'"'
        for kind,name,table,sql in schema:
            if kind!=b'table':continue
            columns=[row[1] for row in c.execute('PRAGMA table_info('+quote(name)+')')];selectors=','.join(['typeof('+quote(col)+')' for col in columns]+[quote(col) for col in columns]);rows=list(c.execute('SELECT '+selectors+' FROM '+quote(name)));tables.append((name,tuple(sorted(rows,key=repr))))
        result=(tuple(schema),tuple(tables))
    except BaseException as error:primary=error
    errors=[]
    for cleanup in [c.rollback,c.close]:
        try:cleanup()
        except BaseException as error:errors.append(error)
    if primary or errors:raise BaseExceptionGroup('Logical fixture snapshot failed',([primary] if primary else [])+errors)
    return result

def run(binary,go,output):
    """Check sorted complete content, omission/nil, isolation, source refusal/recovery.

    Every HTTP/CLI status and original bytes retained; cleanup failures preserved.
    Parameters are not paths, history is not authority, and errors are not [] .
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f,other=historical_fixture(binary,output);before=logical(f.root);state=f.snapshot();db=f.root/'.swarm/state.db';s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None;calls=[];checks=[];go_calls=[];csrf=None
    def request(work=None,status=200,method='GET',raw_query=None,body=None,authorized=True):
        path='/api/v1/exchanges';q=raw_query if raw_query is not None else urllib.parse.urlencode({'work':work or f.work});path+='?'+q;headers={'Cookie':s.cookie,'Origin':'http://'+s.address} if authorized else {};
        if authorized and method not in ['GET','HEAD']:headers['X-Swarm-CSRF']=csrf
        c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status;mime=r.getheader('Content-Type','')
        finally:c.close()
        v=(json.loads(raw) if mime.split(';',1)[0].strip().lower()=='application/json' else raw.decode()) if raw else None;calls.append(dict(path=path,method=method,status=actual,content_type=mime,value=clean(v)));publish(output/'http.json',calls);assert actual==status,(actual,clean(v));return v
    def go_read(work,success=True):
        argv=[str(go),'exchange','list',work,'--root',str(f.root),'--json'];r=subprocess.run(argv,env=f.environment,capture_output=True);n=len(go_calls);publish(output/(f'go-{n}.stdout'),r.stdout);publish(output/(f'go-{n}.stderr'),r.stderr);go_calls.append(dict(argv=argv,exit=r.returncode));publish(output/'go-commands.json',go_calls);assert (r.returncode==0)==success,(r.returncode,r.stderr);return json.loads(r.stdout) if success else None
    def unchanged():assert logical(f.root)==before and f.snapshot()==state and not list(f.root.rglob('model-received.json'))
    try:
        c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request('GET','/api/v1/native-session',headers={'Cookie':s.cookie});r=c.getresponse();assert r.status==200;csrf=json.loads(r.read())['csrf_token']
        finally:c.close()
        native=f.ok(['exchange','list',f.work]);actual=request();assert actual==native==go_read(f.work);assert [e['id'] for e in actual]==['a-first','z-last'];assert actual[0]['artifacts']==[dict(path='',sha256=''),dict(path='first',sha256='declared')] and actual[1]['artifacts'] is None
        assert all(not any(k in e for k in ['recipient_attempt_id','reply_to','deadline_at','acknowledged_at','acknowledged_by','acknowledgement','result_state']) for e in actual);assert actual[0]['need'].startswith('<img ');unchanged();checks.append('actual-go-native-http-exact-sorted-content-omitempty-null-artifacts-no-mutation')
        # Actual Go-compatible SQLite storage and string-wire variants.
        with sqlite3.connect(db) as c:original_artifacts=c.execute('SELECT artifacts FROM agent_exchanges WHERE id=?',('a-first',)).fetchone()[0]
        variants=[('text',original_artifacts),('blob',b'[{"path":"bad-\xff-utf8","sha256":"declared"}]'),('text',b'[{"path":"bad-\xff-utf8","sha256":"declared"}]'),('blob',b'[{"path":"bad-\\ud800-utf16","sha256":"declared"}]')]
        variants+=[('blob',b'[{"path":"bad-\\udc00-utf16","sha256":"declared"}]'),('blob',b'[{"path":"paired-\\ud83d\\ude00","sha256":"declared"}]'),('blob',b'[{"path":"valid","sha256":"hash-\xff"}]'),('text',b'[{"\xff":"ignored","path":"valid","sha256":"declared"}]')]
        for storage,payload in variants:
            # Positive control with identical fixture prerequisites before change.
            assert request()==actual and go_read(f.work)==actual
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_exchanges SET artifacts='+('CAST(? AS TEXT)' if storage=='text' else '?')+' WHERE id=?',(payload,'a-first'))
            variant_before=logical(f.root);native_variant=f.ok(['exchange','list',f.work]);assert request()==native_variant==go_read(f.work);assert logical(f.root)==variant_before
            if b'paired-' in payload:assert native_variant[0]['artifacts'][0]['path']=='paired-'+chr(0x1f600)
            if b'hash-' in payload:assert native_variant[0]['artifacts'][0]['sha256']=='hash-'+chr(0xfffd)
            if b'bad-' in payload:assert native_variant[0]['artifacts'][0]['path']=='bad-'+chr(0xfffd)+('-utf8' if b'utf8' in payload else '-utf16')
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_exchanges SET artifacts=? WHERE id=?',(original_artifacts,'a-first'))
            unchanged()
        checks.append('sqlite-text-blob-and-go-lossy-string-wire-compatible-recovery')
        assert request(other)==f.ok(['exchange','list',other])==go_read(other)==[];assert request(method='HEAD') is None;head=assert_head_wire(s.host,s.port,'/api/v1/exchanges?'+urllib.parse.urlencode({'work':f.work}),s.cookie,observe=lambda phase:publish(output/'head-wire-progress.json',phase));publish(output/'head-wire.json',head);unchanged();checks.append('work-scope-empty-control-head-without-visits-or-consumption')
        request(status=405,method='POST');request(status=403,authorized=False);request(status=400,body=b'{}');request(status=400,raw_query='work='+f.work+'&work='+other);request(status=400,raw_query='work='+f.work+'&unknown=x');request(status=400,raw_query='work=../outside');request(status=400,raw_query='');unchanged();checks.append('session-method-body-unknown-duplicate-unsafe-query-refused-without-mutation')
        assert f.call(['exchange','list','unknown-work'])[0]!=0;go_read('unknown-work',False);request('unknown-work',status=404);unchanged();checks.append('unknown-work-never-successful-empty-array')
        # Every source failure has an immediately preceding valid Go/native control.
        with sqlite3.connect(db) as c:original=c.execute('SELECT artifacts FROM agent_exchanges WHERE id=?',('a-first',)).fetchone()[0]
        for bad in [b'"PRIVATE_BAD_ARTIFACT_FIXTURE"',b'', '',7,1.25]:
            assert request()==actual and go_read(f.work)==actual
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_exchanges SET artifacts=? WHERE id=?',(bad,'a-first'))
            fault=logical(f.root);assert f.call(['exchange','list',f.work])[0]!=0;go_read(f.work,False);error=request(status=500);assert 'PRIVATE_BAD_ARTIFACT_FIXTURE' not in json.dumps(error);assert logical(f.root)==fault
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_exchanges SET artifacts=? WHERE id=?',(original,'a-first'))
            assert request()==actual and go_read(f.work)==actual;unchanged()
        # SQL NULL is forbidden by the retained valid schema, not a valid empty list.
        with sqlite3.connect(db) as c:
            try:c.execute('UPDATE agent_exchanges SET artifacts=NULL WHERE id=?',('a-first',))
            except sqlite3.IntegrityError:pass
            else:raise AssertionError('Retained artifact NOT NULL invariant missing')
        unchanged();checks.append('valid-controls-malformed-empty-numeric-artifacts-safe-refusal-schema-null-and-recovery')
        for language in ['fr','en']:
            r=subprocess.run([str(binary),'--help','--lang',language],capture_output=True);assert r.returncode==0 and b'exchange list WORK' in r.stdout and b'run-limits performance' in r.stdout
        checks.append('actual-fr-en-cli-discovery-for-exchanges-and-graph-policy')
    except BaseException as e:primary=e
    cleanup=None
    try:s.cleanup()
    except BaseException as e:cleanup=e
    if primary or cleanup:raise BaseExceptionGroup('Exchange read and owned cleanup failed',([primary] if primary else [])+([cleanup] if cleanup else []))
    assert len(checks)==7 and not s.capture_errors;publish(output/'result.json',dict(status='PASS',checks=checks,native_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),provider_started=False,exchange_send=False,live_database=False));print('AGENT_EXCHANGES_PUBLIC_GO_VERIFIED 7',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
