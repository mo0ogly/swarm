#!/usr/bin/env python3
"""Historical agent-log HTTP/UX journey on an owned public-CLI fixture.
One inert local executable emits230frames to exercise captured-receipt escaping.
These are not230lifecycle log rows; HTTP compares actual retained CLI pages.
No SQL fixture writes; production history is never used. No global watchdog.
"""
import argparse,hashlib,json,signal,subprocess,threading,os,http.client,sqlite3
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native


def main():
    """Witness exact CLI pages before refusals; verify unchanged records/budget,
    public association/cursor admission and browser states. Preserve original
    outputs and every cleanup error; a failed prerequisite stops dependent work.
    """
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args()
    binary=Path(a.binary).resolve();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    owned=fixture.Fixture(binary,out,'logs-web')
    program=fixture.PROGRAM.replace("print(json.dumps({'type':'result'", "for i in range(230):\n    print(json.dumps({'type':'assistant','message':{'model':model,'content':[{'type':'text','text':'log witness '+str(i)+' <b>literal</b>'}]}}),flush=True)\nprint(json.dumps({'type':'result'")
    fixture.publish(owned.root/'codex',program.encode());os.chmod(owned.root/'codex',0o700)
    prepared=owned.prepare();owned.run_and_assert(prepared)
    other=owned.ok(['work','create'],dict(schema_version=1,event_id='empty-work',expected_revision=0,title='Empty log mission',objective='Read only',scope='Owned fixture',criteria=['No agents']))['work']['id']
    def stored_agent():
        """Read exact persisted body/status/desired through a read-only connection.
        Agent-show heartbeat age is a computed observation, not a stored mutation.
        Never write SQL or use a live Root; this witness covers the owned fixture.
        """
        with sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            return db.execute("SELECT status,desired,CAST(body AS BLOB) FROM agents WHERE id='session'").fetchone()
    server=native.Server(binary,owned.root,out,'logs-web',environment=owned.environment);primary=None;errors=[]
    def drain():
        """Consume owned observer frames through actual stopped event only."""
        try:
            while server.record()['event']!='web-stopped':pass
        except BaseException as e:errors.append(e)
    t=threading.Thread(target=drain);t.start();checks=[]
    def check(name,value):
        assert value,name
        checks.append(name);print('PASS logs-http-'+name,flush=True)
    try:
        before=owned.ok(['work','show',owned.work]);agent_before=owned.ok(['agent','show','session']);agent_bytes=stored_agent();budget=server.request('/api/v1/work-budget?work='+owned.work,observe=False)
        base='/api/v1/agent-logs?work='+owned.work+'&agent=session&after='
        pages={}
        for cursor in [-1,0,9223372036854775807]:
            actual=server.request(base+str(cursor),observe=False);cli=owned.ok(['agent','logs','session',str(cursor)])
            check('CLI-exact-'+str(cursor),actual==dict(schema_version=1,work_id=owned.work,agent_id='session',after=cursor,rows=cli));pages[str(cursor)]=actual
        check('actual-lifecycle-log-witness',len(pages['-1']['rows'])>1 and any('Processus récolté' in r['message'] for r in pages['0']['rows']))
        after=pages['-1']['rows'][0]['seq'];continued=server.request(base+str(after),observe=False)
        check('forward-no-repeated-row',continued['rows']==owned.ok(['agent','logs','session',str(after)]) and continued['rows'] and continued['rows'][0]['seq']>after)
        for name,suffix in [('plus','%2B1'),('leading-zero','01'),('negative-zero','-0'),('overflow','9223372036854775808'),('empty',''),('noninteger','1.1'),('duplicate','0&after=1'),('unknown','0&extra=x')]:
            v=server.request(base+suffix,status=400,observe=False);check(name,v['failure']['code']=='agent-logs-query-invalid')
        for name,path in [('missing-agent','/api/v1/agent-logs?work='+owned.work+'&after=0'),('unsafe-agent','/api/v1/agent-logs?work='+owned.work+'&agent=../escape&after=0')]:
            v=server.request(path,status=400,observe=False);check(name,v['failure']['code']=='agent-logs-query-invalid')
        mismatch=server.request('/api/v1/agent-logs?work='+other+'&agent=session&after=0',status=400,observe=False);check('cross-Work-refused',mismatch['failure']['code']=='agent-logs-work-mismatch')
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        server.request(base+'0',authorized=False,status=403,observe=False);server.request(base+'0',body={},status=403,observe=False);server.request(base+'0',body={},csrf=csrf,status=405,observe=False)
        for method,body,expected in [('HEAD',None,200),('GET',b'{}',400)]:
            connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
            try:
                connection.request(method,base+'0',body=body,headers={'Cookie':server.cookie,'Origin':'http://'+server.address})
                response=connection.getresponse();raw=response.read()
                check(method+'-body-contract',response.status==expected and (raw==b'' if method=='HEAD' else json.loads(raw)['failure']['code']=='agent-logs-query-invalid'))
            finally:connection.close()
        check('read-after-refusals',server.request(base+'0',observe=False)==pages['0'])
        config=dict(address=server.address,key=server.key,root=str(owned.root),output=str(out),work=owned.work,other=other,first=pages['-1'],latest=pages['0'])
        with (out/'browser.stdout').open('xb') as stdout,(out/'browser.stderr').open('xb') as stderr:
            result=subprocess.run(['node','tests/rust_agent_logs_ui.cjs'],input=json.dumps(config).encode(),stdout=stdout,stderr=stderr)
        assert result.returncode==0,('logs-browser-failed',str(out/'browser.stderr'))
        check('Work-events-unchanged',owned.ok(['work','show',owned.work])==before)
        after_agent=owned.ok(['agent','show','session']);check('agent-record-unchanged',stored_agent()==agent_bytes and all(after_agent[k]==agent_before[k] for k in ['agent','desired','observed_status']))
        check('budget-unchanged',server.request('/api/v1/work-budget?work='+owned.work,observe=False)==budget)
        fixture.publish(out/'http-result.json',dict(status='PASS',checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),global_watchdog=None,scope='Owned CLI fixture, one inert process; no real provider'))
        print('AGENT_LOGS_HTTP_UI_PASS '+str(len(checks))+' HTTP assertions',flush=True)
    except BaseException as e:primary=e
    finally:
        if server.process.poll() is None and not server.stop_sent:server.process.send_signal(signal.SIGINT);server.stop_sent=True
        t.join()
    if errors:primary=BaseExceptionGroup('Scenario/drain errors',([primary] if primary else [])+errors)
    native.finish(server,primary)

if __name__=='__main__':main()
