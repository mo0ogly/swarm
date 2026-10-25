#!/usr/bin/env python3
"""Rust preparation CLI/API reads on an isolated historical-schema fixture.

Public Rust init/settings/work operations create the Root. Preparations and
versions are explicitly seeded as historical Go-shaped SQL rows, separately from
the Rust writer recipe. This proves readers, not creation/adoption or native
agent authority. SQL fault injections touch only this owned Root, with valid
controls, originals and recovery. No account/provider or global watchdog.
"""
import argparse,hashlib,http.client,json,os,signal,sqlite3,subprocess,tempfile,threading,urllib.parse
from pathlib import Path
from rust_agent_model_route import publish
import rust_native_http_cli as native


class Driver:
    """Owned CLI fixture with original outputs and public policy history."""
    def __init__(self,binary,out):
        """Retain fixture birth and binary hash before public initialization."""
        self.binary=binary;self.out=out;self.root=Path(tempfile.mkdtemp(prefix='swarm-preparation-read-'));self.calls=[]
        publish(out/'birth.json',dict(root=str(self.root),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),authority=False,kind='owned_historical_read_fixture'))
    def call(self,args,value=None):
        """Retain exact argv/inputs/channels; no implicit retry or shell execution."""
        name='cli-'+str(len(self.calls)+1);argv=[str(self.binary),*args,'--root',str(self.root),'--json']
        if value is not None:
            path=self.out/(name+'.input.json');publish(path,value);argv+=['--input',str(path)]
        result=subprocess.run(argv,stdin=subprocess.DEVNULL,capture_output=True)
        publish(self.out/(name+'.stdout'),result.stdout);publish(self.out/(name+'.stderr'),result.stderr)
        self.calls.append(dict(argv=argv,exit=result.returncode));publish(self.out/'commands.json',self.calls)
        return result.returncode,result.stdout,result.stderr
    def ok(self,args,value=None):
        """Decode meaningful CLI success, retaining every original failure."""
        code,out,err=self.call(args,value);assert code==0,(args,code,err.decode());assert not err,err.decode();return json.loads(out)
    def settings(self,**updates):
        """Preview/apply only supplied policy fields; preserve preview bytes and history."""
        doc=self.ok(['engine-settings','show']);values=doc['values'];values.update(updates)
        change=dict(schema_version=self.ok(['engine-settings','describe'])['document_schema_version'],event_id='prep-settings-'+str(len(self.calls)),expected_revision=doc['revision'],values=values,reason='Owned preparation read policy witness',preview_token='')
        original=(self.root/'.swarm/engine-settings.json').read_bytes();preview=self.ok(['engine-settings','preview'],change)
        assert (self.root/'.swarm/engine-settings.json').read_bytes()==original
        change['preview_token']=preview['preview_token'];self.ok(['engine-settings','apply'],change)
        assert self.ok(['engine-settings','show'])['values']==values
    def database(self):
        """Hash the complete owned persisted SQL dump; never use a production Root."""
        with sqlite3.connect('file:'+str(self.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    def write_fixture(self,sql,params):
        """Explicit historical/fault fixture write, not a public Rust writer proof."""
        with sqlite3.connect(self.root/'.swarm/state.db') as db:
            db.execute('PRAGMA foreign_keys=ON');db.execute(sql,params)


def main():
    """Compare public reads, fault/refusal/recovery and actual Admin consumption.
    Preserve original failure channels and stop the owned server even on failure.
    """
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    binary=Path(a.binary).resolve();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(binary,out);checks=[]
    def check(name,value):
        assert value,name;assert name not in checks,name;checks.append(name);print('PASS preparations-'+name,flush=True)
    d.ok(['init']);work=d.ok(['work','create'],dict(schema_version=1,event_id='prep-work',expected_revision=0,title='Preparation read fixture',objective='Observe stored preparations',scope='Owned historical fixture',criteria=['No execution or adoption']))['work']['id']
    baseline=d.database();empty=d.ok(['prepare','list']);check('empty-explicit',empty==dict(preparations=[],limit=100));check('empty-read-no-write',d.database()==baseline)
    methods=d.ok(['prepare','methods']);check('packaged-methods-valid',len(methods)==5 and all(m['available'] and len(m['sha256'])==64 for m in methods));method=next(m for m in methods if m['id']=='apex')
    docs={}
    for kind in ['besoin','brief','plan']:
        text='Owned '+kind+' <b>literal</b>';docs[kind]=dict(kind=kind,text=text,revision=3,sha256=hashlib.sha256(text.encode()).hexdigest(),at='2026-10-24T00:00:00Z')
    adopted={**docs['brief'],'need_hash':docs['besoin']['sha256']}
    originals={}
    for identifier,title in [('p-first','First literal <b>'),('p-second','Second'),('p-last','Last')]:
        p=dict(id=identifier,title=title,method='apex',method_hash=method['sha256'],revision=3,created_at='2026-10-24T00:00:00Z',updated_at='2026-10-24T00:00:00Z',documents=docs,adopted_brief=adopted,verdict=dict(plan_hash=docs['plan']['sha256'],brief_hash=docs['brief']['sha256'],method_hash=method['sha256'],at='2026-10-24T00:00:00Z'),plan_ready=False)
        raw=json.dumps(p).encode();originals[identifier]=raw;d.write_fixture('INSERT INTO preparations(id,work_id,revision,body) VALUES(?,NULL,3,?)',(identifier,raw))
    history=[]
    for revision in [1,2,3]:
        doc={**docs['plan'],'revision':revision,'text':'Retained version '+str(revision)};raw=json.dumps(doc).encode();history.append(doc)
        d.write_fixture('INSERT INTO preparation_documents(preparation_id,kind,revision,body) VALUES(?,?,?,?)',('p-first','plan',revision,raw))
    publish(out/'historical-fixture.json',dict(preparations={k:json.loads(v) for k,v in originals.items()},history=history,writer='explicit isolated SQL historical rows, not Rust writer'))
    before=d.database();server=native.Server(binary,d.root,out,'preparations');primary=None;errors=[]
    def drain():
        """Consume only the owned HTTP observer through its stopped event."""
        try:
            while server.record()['event']!='web-stopped':pass
        except BaseException as error:errors.append(error)
    thread=threading.Thread(target=drain);thread.start()
    def same(label,args,path):
        """Assert exact CLI/HTTP DTO agreement, storing both original observations."""
        cli=d.ok(args);http=server.request(path,observe=False);publish(out/(label+'.json'),dict(cli=cli,http=http));check(label+'-CLI-HTTP',cli==http);return cli
    try:
        listed=same('list',['prepare','list'],'/api/v1/preparations');check('recorded-row-order',[p['id'] for p in listed['preparations']]==['p-last','p-second','p-first'])
        shown=same('show',['prepare','show','p-first'],'/api/v1/preparations/show?id=p-first');check('freshness-recomputed',shown['plan_ready'] is True and json.loads(originals['p-first'])['plan_ready'] is False and shown['title']=='First literal <b>')
        same('methods',['prepare','methods'],'/api/v1/preparations/methods')
        full=same('history',['prepare','history','p-first','plan'],'/api/v1/preparations/history?id=p-first&document=plan');check('history-actual-content',full==history[::-1])
        older=same('history-before',['prepare','history','p-first','plan','3'],'/api/v1/preparations/history?id=p-first&document=plan&before=3');check('history-exclusive',older==history[1::-1])
        import rust_preparation_queries
        rust_preparation_queries.verify_queries(server,'p-first',check)
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];server.request('/api/v1/preparations',body={},csrf=csrf,status=405,observe=False)
        for verb,body,status in [('HEAD',None,200),('GET',b'{}',400)]:
            connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
            try:
                connection.request(verb,'/api/v1/preparations',body=body,headers={'Cookie':server.cookie,'Origin':'http://'+server.address});reply=connection.getresponse();data=reply.read();check(verb+'-body-contract',reply.status==status and (data==b'' if verb=='HEAD' else json.loads(data)['failure']['code']=='preparation-query-invalid'))
            finally:connection.close()
        check('reads-refusals-no-write',d.database()==before)
        corrupt=b'{"id":100,"private":"DO_NOT_LEAK_PREPARATION"}'
        d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(corrupt,'p-first'));fault=d.database()
        failure=server.request('/api/v1/preparations/show?id=p-first',status=500,observe=False);code,stdout,stderr=d.call(['prepare','show','p-first'])
        check('stored-codec-refusal-safe',code==2 and failure['failure']['code']=='preparation-document-invalid' and b'DO_NOT_LEAK_PREPARATION' not in stdout+stderr and 'DO_NOT_LEAK_PREPARATION' not in json.dumps(failure));check('corrupt-read-no-repair',d.database()==fault)
        d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(originals['p-first'],'p-first'));check('codec-recovery',server.request('/api/v1/preparations/show?id=p-first',observe=False)==shown and d.database()==before)
        mismatched=json.loads(originals['p-first']);mismatched['id']='p-different';d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(mismatched).encode(),'p-first'));fault=d.database()
        failure=server.request('/api/v1/preparations/show?id=p-first',status=500,observe=False);check('row-binding-refused',failure['failure']['code']=='preparation-row-identity-mismatch' and d.database()==fault)
        d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(originals['p-first'],'p-first'));check('binding-recovery',server.request('/api/v1/preparations/show?id=p-first',observe=False)==shown)
        local=d.root/'.claude/skills/apex/SKILL.md';local.parent.mkdir(parents=True);local_bytes=Path('.claude/skills/apex/SKILL.md').read_bytes();publish(local,local_bytes)
        check('actual-local-control',server.request('/api/v1/preparations/show?id=p-first',observe=False)['plan_ready'] is True)
        local.unlink();os.symlink('missing-method',local);unavailable=server.request('/api/v1/preparations/methods',observe=False);apex=next(m for m in unavailable if m['id']=='apex')
        check('unsafe-override-no-bundled-fallback',apex['available'] is False and apex['reason']=='preparation-method-open' and not apex['sha256']);check('unsafe-method-not-fresh',server.request('/api/v1/preparations/show?id=p-first',observe=False)['plan_ready'] is False)
        local.unlink();publish(local,local_bytes);check('source-recovery',server.request('/api/v1/preparations/show?id=p-first',observe=False)['plan_ready'] is True)
        d.settings(preparation_list_limit='2',preparation_history_limit='1',preparation_method_max_bytes='0')
        limited=same('limited-list',['prepare','list'],'/api/v1/preparations');check('saved-limit-consumed',limited['limit']==2 and len(limited['preparations'])==2 and all(p['plan_ready'] is False for p in limited['preparations']))
        limited_history=same('limited-history',['prepare','history','p-first','plan'],'/api/v1/preparations/history?id=p-first&document=plan');check('history-policy-consumed',limited_history==history[-1:])
        d.settings(preparation_list_limit=None,preparation_history_limit=None,preparation_method_max_bytes=None)
        unlimited=same('unlimited',['prepare','list'],'/api/v1/preparations');check('unlimited-explicit',unlimited['limit'] is None and len(unlimited['preparations'])==3 and all(p['plan_ready'] is True for p in unlimited['preparations']))
        d.settings(preparation_list_limit='9007199254740993');huge=same('large-exact',['prepare','list'],'/api/v1/preparations');check('large-count-no-hidden-cutoff',huge['limit']==9007199254740993 and len(huge['preparations'])==3)
        d.settings(preparation_list_limit='0');zero=same('zero',['prepare','list'],'/api/v1/preparations');check('zero-no-deletion',zero==dict(preparations=[],limit=0) and d.database()==before)
        d.settings(preparation_list_limit='2',preparation_history_limit='50',preparation_method_max_bytes='131072')
        cfg=dict(address=server.address,key=server.key,root=str(d.root),work=work,output=str(out))
        with (out/'browser.stdout').open('xb') as o,(out/'browser.stderr').open('xb') as e:r=subprocess.run(['node','tests/rust_preparation_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=o,stderr=e)
        check('Admin-browser',r.returncode==0)
        actual=same('Admin-consumer',['prepare','list'],'/api/v1/preparations');check('Admin-save-real-consumer',actual['limit']==1 and len(actual['preparations'])==1)
        check('business-data-unchanged',d.database()==before)
        publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),fixture_root=str(d.root),global_watchdog=None,limitations=['Historical SQL fixture, no Rust preparation writer proof','No preparation editor/dialogue/conversion or provider execution']))
        print('PREPARATION_READS_PASS '+str(len(checks))+' assertions',flush=True)
    except BaseException as error:primary=error
    finally:
        if server.process.poll() is None and not server.stop_sent:server.process.send_signal(signal.SIGINT);server.stop_sent=True
        thread.join()
    if errors:primary=BaseExceptionGroup('Preparation/drain failures',([primary] if primary else [])+errors)
    native.finish(server,primary)

if __name__=='__main__':main()
