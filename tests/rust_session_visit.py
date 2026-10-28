#!/usr/bin/env python3
"""Session visit HTTP/CLI on owned Roots, no acceptance/provider/live DB writes.

Reference server startup is isolated from native state; original replies and SQL
visit rows are retained. No arbitrary watchdog, hidden retry or operation refund.
"""
import argparse,json,http.client,os,re,sqlite3,subprocess,traceback,urllib.parse
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical
from rust_native_http_cli import Server,clean

def run(binary,go,output,unported=False):
    """Prove controls before route refusal, and preserve all owned cleanup errors.
    Native operator/time are server-derived, never client-supplied authority.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'visit');before=logical(f.root);gd=output/'go-fixture';gd.mkdir();gf=Fixture(binary,gd,'go-visit');s=None;g=None;primary=None;calls=[]
    def wire(address,cookie,path,status=200,method='GET',csrf=None,body=None):
        """Protected status/content with checked connection close and token redaction."""
        host,port=address.rsplit(':',1);c=http.client.HTTPConnection(host,int(port),timeout=None);headers={'Origin':'http://'+address}
        if cookie:headers['Cookie']=cookie
        if csrf:headers['X-Swarm-CSRF']=csrf
        if body is not None:headers['Content-Type']='application/json'
        try:c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        try:value=json.loads(raw) if raw else None
        except (UnicodeDecodeError,json.JSONDecodeError):value=raw.decode(errors='replace')
        calls.append(dict(path=path,method=method,status=actual,value=clean(value)));publish(output/'http.json',calls);assert actual==status,(actual,clean(value));return value
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(gf.root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=gf.environment);line=g.stdout.readline().decode();m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',line.strip());assert m;address,token=m.groups();assert wire(address,'swarm_session='+token,'/api/v1/works');saved=wire(address,'swarm_session='+token,'/api/v1/visit?'+urllib.parse.urlencode(dict(work=gf.work)),method='POST',csrf=token);assert saved=={'saved':True};g.terminate();_,err=g.communicate();assert token.encode() not in err;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(err)));g=None
        with sqlite3.connect((gf.root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True) as c:
            rows=list(c.execute('SELECT operator,revision,at FROM session_visits WHERE work_id=?',(gf.work,)));revision=c.execute('SELECT revision FROM works WHERE id=?',(gf.work,)).fetchone()[0]
        assert len(rows)==1 and rows[0][0] and rows[0][1]==revision and rows[0][2];publish(output/'go-visit.json',dict(operator=rows[0][0],revision=rows[0][1],at=rows[0][2]))
        s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');assert wire(s.address,s.cookie,'/api/v1/works');csrf=wire(s.address,s.cookie,'/api/v1/native-session')['csrf_token'];wire(s.address,s.cookie,'/api/v1/visit?'+urllib.parse.urlencode(dict(work=f.work)),404 if unported else 200,method='POST',csrf=csrf)
        if unported:assert logical(f.root)==before
        else:
            path='/api/v1/visit?'+urllib.parse.urlencode(dict(work=f.work));read_path='/api/v1/visit-state?'+urllib.parse.urlencode(dict(work=f.work))
            def unchanged_business():
                """Exclude only the intended auxiliary table; compare schema/all other rows."""
                snapshot=logical(f.root)
                assert snapshot[0]==before[0]
                assert tuple(t for t in snapshot[1] if t[0]!=b'session_visits')==tuple(t for t in before[1] if t[0]!=b'session_visits')
            def revision_intent(revision):return json.dumps(dict(schema_version=1,expected_revision=str(revision))).encode()
            state=wire(s.address,s.cookie,read_path);cli=f.ok(['visit','show',f.work]);assert cli==state and state['exists'] is True and str(state['visit']['revision'])==state['exact_visit_revision']==state['work_revision']
            revision=int(state['work_revision']);assert state['visit']['operator'] and state['visit']['at'];unchanged_business()
            before_refusal=logical(f.root)
            wire(s.address,s.cookie,path,409,method='POST',csrf=csrf,body=revision_intent(revision-1));assert logical(f.root)==before_refusal
            for invalid in [b'{}',b'{"schema_version":1,"expected_revision":null}',b'{"schema_version":1,"expected_revision":1}',b'{"schema_version":1,"expected_revision":"01"}',b'{"schema_version":1,"schema_version":1}',b'{"schema_version":1,"operator":"forged"}']:
                wire(s.address,s.cookie,path,400,method='POST',csrf=csrf,body=invalid);assert logical(f.root)==before_refusal
            wire(s.address,s.cookie,path,405);wire(s.address,s.cookie,read_path,400,body=b'{}');wire(s.address,s.cookie,read_path+'&work='+f.work,400);wire(s.address,s.cookie,path,403,method='POST',body=b'');wire(s.address,None,read_path,403);assert logical(f.root)==before_refusal
            marked=wire(s.address,s.cookie,path,method='POST',csrf=csrf,body=revision_intent(revision));assert marked['saved'] is True and marked['state']==wire(s.address,s.cookie,read_path)
            assert f.ok(['visit','mark',f.work],dict(schema_version=1,expected_revision=str(revision)))['saved'] is True
            code,refused=f.call(['visit','mark',f.work],dict(schema_version=1,expected_revision=str(revision-1)));assert code!=0 and 'visit-revision-conflict' in json.dumps(refused)
            # Actual CLI output failure after a valid saved write, not a mocked
            # receipt. Only an owned pipe loses its reader; no process is killed.
            reader,writer=os.pipe();os.close(reader)
            try:
                result=subprocess.run([str(binary),'visit','mark',f.work,'--root',str(f.root),'--json'],env=f.environment,stdin=subprocess.DEVNULL,stdout=writer,stderr=subprocess.PIPE)
            finally:os.close(writer)
            publish(output/'broken-output.stderr',result.stderr);assert result.returncode!=0
            failed=json.loads(result.stderr);assert failed['auxiliary_effects'] and any(e['kind']=='session_visit' and e['state']=='recorded' and e['work_id']==f.work for e in failed['auxiliary_effects'])
            assert f.ok(['visit','show',f.work])['exists'] is True
            unchanged_business()
            # Existing valid bookmark is the control. Corrupt only this owned
            # fixture row, then require authoritative refusal and public repair.
            with sqlite3.connect(f.root/'.swarm/state.db') as c:c.execute('UPDATE session_visits SET revision=-1 WHERE work_id=?',(f.work,))
            invalid_before=logical(f.root);wire(s.address,s.cookie,read_path,500);assert logical(f.root)==invalid_before
            assert wire(s.address,s.cookie,path,method='POST',csrf=csrf,body=revision_intent(revision))['saved'] is True
            assert wire(s.address,s.cookie,read_path)['exact_visit_revision']==str(revision)
            # Added SQL triggers are refused by public Root schema admission;
            # they do not exercise the inner changed-row/readback branches.
            db=f.root/'.swarm/state.db'
            with sqlite3.connect(db) as c:c.execute("CREATE TRIGGER visit_test_ignore BEFORE INSERT ON session_visits BEGIN SELECT RAISE(IGNORE); END")
            refused_before=logical(f.root);reply=wire(s.address,s.cookie,path,500,method='POST',csrf=csrf,body=revision_intent(revision));assert 'root-schema-mismatch' in json.dumps(reply) and not reply.get('auxiliary_effects');assert logical(f.root)==refused_before
            with sqlite3.connect(db) as c:c.execute('DROP TRIGGER visit_test_ignore')
            assert wire(s.address,s.cookie,path,method='POST',csrf=csrf,body=revision_intent(revision))['saved'] is True
            unchanged_business();publish(output/'assertions.json',dict(protocol=True,exact_receipt=True,cli_http_parity=True,stale_refusal=True,invalid_refusal=True,method_auth_csrf=True,schema_fault_refusal_recovery=True,actual_broken_output_receipt=True,invalid_stored_revision_public_repair=True,business_state_unchanged=True))
        assert not list(f.root.rglob('model-received.json')) and not list(gf.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    errors=[]
    if g is not None:
        try:g.terminate();g.communicate()
        except BaseException as e:errors.append(e)
    if s is not None:
        try:s.cleanup()
        except BaseException as e:errors.append(e)
    if primary:errors.insert(0,primary)
    if errors:raise BaseExceptionGroup('Visit and owned cleanup',errors)
    if s:assert not s.capture_errors
    if not unported:
        observed=[json.loads(line) for line in (output/'stream.jsonl').read_text().splitlines()]
        receipts=[row['outcome']['auxiliary_effects'] for row in observed if row.get('event')=='web-outcome' and row['outcome'].get('auxiliary_effects')]
        assert receipts and all(e['kind']=='session_visit' and e['state']=='recorded' and e['work_id']==f.work for group in receipts for e in group)
        for row in observed:
            if row.get('event')=='web-outcome' and row['outcome'].get('auxiliary_effects'):
                assert row['outcome'].get('recorded_work') is None and row['outcome'].get('configuration_may_be_recorded') is False
        stopped=[row for row in observed if row.get('event')=='web-stopped'];assert len(stopped)==1 and stopped[0]['last_auxiliary_effects']==receipts[-1]
        assert stopped[0]['work_outcomes']==0 and stopped[0]['configuration_outcomes']==0
    publish(output/'result.json',dict(status='PASS',scope='Go visit positive and native route absence' if unported else 'Go differential, native HTTP/CLI exact state, refusal, rollback, recovery and business preservation',binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),provider_started=False,live_database=False));print('SESSION_VISIT_UNPORTED_WITNESS' if unported else 'SESSION_VISIT_VERIFIED 10',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
