#!/usr/bin/env python3
"""Exercise documentary capture/edit/list contracts on an isolated inactive Root.

No live database or provider is used. Fixtures include malicious paths and secret
shapes made from synthetic text; no real credential is read. Each refusal follows
a valid witness and must preserve SQL. No elapsed-time watchdog is imposed.
"""
import argparse,hashlib,json,os,sqlite3
from pathlib import Path
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_native_http_cli as native
from rust_preparation_plan import plan

def main():
    """Require saved policy consumption, stale-hash refusal, replay and recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'preparation-sources');f.settings_change(preparation_write_enabled='1');checks=[]
    def check(name,value):
        """Retain each named observation immediately; failures remain failures."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='sources-create',expected_revision=0,title='Source fixture',text='Need'))
    docs=f.root/'docs';docs.mkdir();(docs/'a.md').write_bytes(b'first\r\nsecond\n');(docs/'b.txt').write_text('another');(docs/'c.xyz').write_text('custom');(docs/'nested').mkdir();(docs/'nested'/'d.md').write_text('nested')
    def capture(path='docs/a.md',start=1,end=3):return f.ok(['prepare','source',p['id'],path,str(start),str(end)])
    def listing(scope='docs',q=''):return f.ok(['prepare','files',p['id'],scope,q])
    def dump():
        """Fingerprint complete test SQL without changing it."""
        with sqlite3.connect(f.root/'.swarm/state.db') as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    def refused(name,args,expected=None,value=None):
        """Prove a valid control first, then refusal without SQL mutation."""
        capture();before=dump();code,reply=f.call(args,value);check(name,code!=0 and before==dump() and (expected is None or reply['failure']['code']==expected));return reply
    source=capture();check('source-exact-bytes-range-full-digest',source['text']=='first\r\nsecond\n' and source['end_line']==3 and source['sha256']==hashlib.sha256((docs/'a.md').read_bytes()).hexdigest());check('source-clips-end',capture(end=20)['end_line']==3)
    view=listing();check('source-list-lexical-recursive',view['directories']==['docs/nested'] and view['paths']==['docs/a.md','docs/b.txt','docs/nested/d.md'] and not view['limited']);check('source-query-case-insensitive',listing(q='NESTED')['paths']==['docs/nested/d.md'])
    for name,path in [('traversal','../a.md'),('absolute',str(docs/'a.md')),('private','.swarm/engine-settings.json'),('private-name','docs/settings.json'),('scalar-case-private','docs/credentİals.md'),('type','docs/c.xyz')]:
        refused('source-refuses-'+name,['prepare','source',p['id'],path,'1','1'])
    (docs/'link.md').symlink_to(docs/'a.md');(f.root/'linked').symlink_to(docs,target_is_directory=True);os.mkfifo(docs/'fifo.md')
    for name,path in [('symlink','docs/link.md'),('parent-symlink','linked/a.md'),('fifo','docs/fifo.md')]:refused('source-refuses-'+name,['prepare','source',p['id'],path,'1','1'])
    for name,raw in [('binary',b'hello\x00world'),('utf8',b'\xff'),('secret-outside-excerpt',b'innocent\npassword: abcdef\n'),('secret-unicode-scalars','password: éééééé'.encode())]:
        (docs/'invalid.md').write_bytes(raw);refused('source-refuses-'+name,['prepare','source',p['id'],'docs/invalid.md','1','1'])
    (docs/'invalid.md').unlink();(docs/'short.md').write_text('password: ééééé');check('source-secret-counts-scalars',capture('docs/short.md',1,1)['text']=='password: ééééé');(docs/'short.md').unlink()
    defaults=f.ok(['engine-settings','show'])['values'];check('source-group-saved-initial-defaults',all(k in defaults for k in ['preparation_source_file_max_bytes','preparation_source_excluded_components']) and 'private' in defaults['preparation_source_excluded_components'])
    for key,change,argv in [
        ('preparation_source_file_max_bytes','1',['prepare','source',p['id'],'docs/a.md','1','3']),
        ('preparation_source_max_bytes','1',['prepare','source',p['id'],'docs/a.md','1','3']),
        ('preparation_source_max_lines','1',['prepare','source',p['id'],'docs/a.md','1','3']),
        ('preparation_source_path_max_bytes','1',['prepare','source',p['id'],'docs/a.md','1','3']),
        ('preparation_source_query_max_bytes','1',['prepare','files',p['id'],'docs','nested'])]:
        capture();f.settings_change(**{key:change});before=dump();code,_=f.call(argv);check('source-consumes-'+key,code!=0 and before==dump());f.settings_change(**{key:defaults[key]});check('source-recovers-'+key,capture()['sha256']==source['sha256'])
    for key in ['preparation_source_directory_limit','preparation_source_file_limit','preparation_source_scan_limit']:
        f.settings_change(**{key:'0'});view=listing();check('source-zero-'+key,view['limited'] and (key!='preparation_source_file_limit' or not view['paths']) and (key!='preparation_source_directory_limit' or not view['directories']));f.settings_change(**{key:None});check('source-unlimited-'+key,not listing()['limited']);f.settings_change(**{key:defaults[key]})
    f.settings_change(preparation_source_allowed_extensions=['.xyz']);check('source-extension-policy-consumed',listing()['paths']==['docs/c.xyz'] and capture('docs/c.xyz',1,1)['text']=='custom');f.settings_change(preparation_source_allowed_extensions=defaults['preparation_source_allowed_extensions']);f.settings_change(preparation_source_excluded_components=['nested']);check('source-exclusions-policy-consumed','docs/nested/d.md' not in listing()['paths']);f.settings_change(preparation_source_excluded_components=defaults['preparation_source_excluded_components'])
    def request(event,action,reference):return dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],source=reference)
    def edit_document(action,event,**fields):
        """Prepare a valid adopted/verdict witness before source invalidation."""
        return f.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**fields))
    p=edit_document('save','source-brief',document='brief',text='Retained brief');p=edit_document('adopt-brief','source-adopt',sha256=p['documents']['brief']['sha256']);p=edit_document('save','source-plan',document='plan',text=json.dumps(plan()));p=edit_document('validate-plan','source-verdict',sha256=p['documents']['plan']['sha256']);documents=p['documents'];check('source-adoption-verdict-valid-control',bool(p.get('adopted_brief')) and bool(p.get('verdict')) and p['plan_ready'])
    ref={k:source[k] for k in ['path','start_line','end_line','sha256']};req=request('source-add','source-add',ref);before=dump();(docs/'a.md').write_text('changed');code,reply=f.call(['prepare','source-add',p['id']],req);check('source-stale-refuses-no-mutation',code!=0 and reply['failure']['code']=='preparation-stale-source' and before==dump());(docs/'a.md').write_bytes(b'first\r\nsecond\n')
    p=f.ok(['prepare','source-add',p['id']],req);check('source-add-engine-snapshot',p['sources'][0]['text']==source['text'] and p['sources'][0]['sha256']==source['sha256'] and not p.get('plan_ready') and not p.get('adopted_brief') and not p.get('verdict') and p['documents']==documents)
    before=dump();(docs/'a.md').unlink();replay=f.ok(['prepare','source-add',p['id']],req);check('source-exact-replay-without-file',p==replay and before==dump());(docs/'a.md').write_bytes(b'first\r\nsecond\n')
    f.settings_change(preparation_sources_max_count='1');other=capture('docs/b.txt',1,1);other_ref={k:other[k] for k in ['path','start_line','end_line','sha256']};refused('source-count-limit-no-mutation',['prepare','source-add',p['id']],'preparation-source-count-over-limit',request('count','source-add',other_ref));f.settings_change(preparation_sources_max_count=None,preparation_sources_max_bytes='1');refused('source-total-limit-no-mutation',['prepare','source-add',p['id']],'preparation-source-total-over-limit',request('total','source-add',other_ref))
    f.settings_change(preparation_source_file_max_bytes='0',preparation_source_max_bytes='0',preparation_sources_max_count='0');(docs/'a.md').unlink();remove=request('source-remove','source-remove',dict(path=ref['path']));p=f.ok(['prepare','source-remove',p['id']],remove);check('source-remove-under-denying-policy-missing-file',not p.get('sources'));before=dump();check('source-remove-replay',f.ok(['prepare','source-remove',p['id']],remove)==p and before==dump())
    f.settings_change(**{k:v for k,v in defaults.items() if k.startswith('preparation_source')});(docs/'a.md').write_bytes(b'first\r\nsecond\n')
    server=native.Server(f.binary,f.root,out,'preparation-sources',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        query=urlencode(dict(id=p['id'],path='docs/a.md',start=1,end=3));before=dump();http=server.request('/api/v1/preparations/source?'+query,observe=False);check('source-http-cli-content',http['text']==capture()['text'] and http['sha256']==source['sha256']);check('source-http-list-cli-equal',server.request('/api/v1/preparations/files?'+urlencode(dict(id=p['id'],scope='docs',q='')),observe=False)==listing());server.request('/api/v1/preparations/source?'+query+'&start=1',status=400,observe=False);check('source-http-duplicate-refuses-no-write',before==dump())
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    check('source-no-provider-execution',not (f.root/'model-received.json').exists());print('PREPARATION_SOURCES_PASS '+str(len(checks)))
if __name__=='__main__':main()
