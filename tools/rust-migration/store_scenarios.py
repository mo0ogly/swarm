"""Actual isolated Store CLI scenarios. No timers, provider or mission operations."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess

class Client:
    def __init__(self,binary,root,output,label,env):
        self.label=label;self.log=(output/(label+'.jsonl')).open('w');self.stderr=(output/(label+'.stderr')).open('wb')
        self.process=subprocess.Popen([str(binary),str(root)],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr)
    def call(self,command):
        self.log.write(json.dumps(dict(input=command))+'\n');self.log.flush()
        self.process.stdin.write(json.dumps(command,separators=(',',':')).encode()+b'\n');self.process.stdin.flush()
        raw=self.process.stdout.readline();assert raw,self.label+': fixture process exited; stderr retained'
        result=json.loads(raw);self.log.write(json.dumps(dict(output=result))+'\n');self.log.flush()
        if result['code']!='ok':assert result['zero'] and result['view']is None and result['projection']is None and result['raw']=='','partial refusal'
        return result
    def close(self):
        if self.process.poll()is None:self.process.stdin.close()
        status=self.process.wait();self.log.close();self.stderr.close();assert status==0,self.label+': fixture exit failed'
    def abort(self):
        if self.process.poll()is None:self.process.terminate()
        self.process.wait();self.log.close();self.stderr.close()

def run(go,rust,work,output,env):
    roots=[work/'go-store',work/'rust-store']
    for p in roots:p.mkdir(mode=0o700)
    clients=[Client(binary,root,output,label,env)for binary,root,label in zip([go,rust],roots,['store-go','store-rust'])]
    cases=[];cross=[]
    def step(name,command,expected='ok',unchanged=False):
        before=[document(p)for p in roots]if unchanged else None
        result=[c.call(command)for c in clients]
        assert result[0]==result[1],name+': Go/Rust outputs differ; original JSONL retained'
        assert result[0]['code']==expected,(name,result[0]['code'],expected)
        if unchanged:assert [document(p)for p in roots]==before,name+': refused/preview operation mutated storage'
        cases.append(name)
        if len(cases)%10==0:print('store scenarios',len(cases),flush=True)
        return result[0]
    def document(root):
        p=root/'main/config.json'
        if not p.exists()and not p.is_symlink():return None
        st=p.lstat();data=None
        if stat.S_ISREG(st.st_mode):
            try:data=hashlib.sha256(p.read_bytes()).hexdigest()
            except PermissionError:data='unreadable'
        return dict(mode=stat.S_IMODE(st.st_mode),type=stat.S_IFMT(st.st_mode),links=st.st_nlink,data=data,link=os.readlink(p)if p.is_symlink()else None)
    def read(head):return step('valid-read-control',dict(op='read',handle='a',head=head))['view']
    zero=dict(revision='0',digest='');at='2026-10-17T12:00:00Z'
    def change(event,head):return dict(schema_version=1,event_id=event,expected_head=head,values=dict(mode='local',local_socket_path='/run/swarm-fixture/authority.sock'),reason='explicit fixture choice')
    def call(op,c,**extra):return dict(op=op,handle='a',change=c,at=at,**extra)
    try:
        step('create-owned-fixture-directory',dict(op='mkdir',dir='main'))
        step('open-valid',dict(op='open',dir='main',handle='a'))
        c0=change('initial',zero)
        preview=step('preview-does-not-create-config',call('preview',c0),unchanged=True)['projection'];assert preview['base']==zero and preview['proposed']['head']['revision']=='1'
        first=step('explicit-create',call('create',c0))['view'];assert first==preview['proposed']and len(first['snapshot']['history'])==1
        assert document(roots[0])==document(roots[1]);head=first['head'];read(head)
        assert step('exact-create-replay',call('create',c0),unchanged=True)['view']==first
        c1=change('second',head)
        step('preview-does-not-write',call('preview',c1),unchanged=True)
        second=step('apply-next-revision',call('apply',c1))['view'];assert second['head']['revision']=='2'and len(second['snapshot']['history'])==2
        head=second['head'];read(head)
        replay=call('apply',c1);replay['at']='2026-10-18T12:00:00Z';assert step('exact-current-replay',replay,unchanged=True)['view']==second
        for name,cmd,expected in [
            ('old-event',call('apply',c0),'event'),('stale-cas',call('apply',change('stale',first['head'])),'conflict'),
            ('old-head',dict(op='read',handle='a',head=first['head']),'head'),('missing-independent-head',dict(op='read',handle='a',head=zero),'head'),
            ('canceled-read',dict(op='read',handle='a',head=head,canceled=True),'canceled'),
            ('deadline-read',dict(op='read',handle='a',head=head,deadline=True),'deadline'),
            ('deadline-apply',call('apply',change('expired',head),deadline=True),'deadline'),
            ('restrictive-umask',call('apply',change('restrictive',head),umask=0o777),'path'),
            ('unrepresentable-preview',call('preview',change('invalid',head),year=10000),'format'),('unrepresentable-apply',call('apply',change('invalid',head),year=10000),'format')]:
            read(head);step(name,cmd,expected,unchanged=True);read(head)
        for authority,expected in [('deny','unauthorized'),('wrong','unauthorized'),('cancel','canceled')]:
            read(head);handle='auth-'+authority;step('open-'+authority,dict(op='open',dir='main',handle=handle,authority=authority))
            cmd=call('apply',change('reject-'+authority,head));cmd['handle']=handle;step('refuse-'+authority,cmd,expected,unchanged=True);read(head)
        for name,options,expected in [('owner',dict(owner_delta=1),'path'),('no-authorizer',dict(authority='none'),'unauthorized')]:
            read(head);step('open-invalid-'+name,dict(op='open',dir='main',handle='invalid-'+name,**options),expected,unchanged=True);read(head)
        step('open-other-independent-pins',dict(op='open',dir='main',handle='other-pins',different_pins=True));step('pins-mismatch',dict(op='read',handle='other-pins',head=head),'pins',unchanged=True);read(head)
        step('owned-callback-handle',dict(op='open',dir='main',handle='owned',authority='owned-copy'))
        cmd=call('preview',change('owned-preview',head));cmd['handle']='owned';step('callback-only-mutates-owned-copy',cmd,unchanged=True);read(head)
        saved=[(p/'main/config.json').read_bytes()for p in roots];assert saved[0]==saved[1]
        for field,mode in [('config.json',0o640),('config.lock',0o644),('',0o755)]:
            read(head)
            for p in roots:(p/'main'/field).chmod(mode)
            step('unsafe-live-permissions-'+field,dict(op='read',handle='a',head=head),'path',unchanged=True)
            for p in roots:(p/'main'/field).chmod(0o700 if not field else 0o600)
            read(head)
        for kind in ['hardlink','symlink','fifo','rollback','malformed','whitespace']:
            read(head)
            for p,raw in zip(roots,saved):
                path=p/'main/config.json'
                if kind=='hardlink':os.link(path,p/'main/hard')
                elif kind=='symlink':
                    outside=p/'main/outside';outside.write_bytes(raw);outside.chmod(0o600);path.unlink();path.symlink_to(outside)
                elif kind=='fifo':path.unlink();os.mkfifo(path,0o600)
                elif kind=='rollback':path.write_text(json.dumps(first['snapshot'],separators=(',',':')))
                elif kind=='malformed':path.write_bytes(b'{}')
                elif kind=='whitespace':path.write_bytes(b' '+raw)
            expected={'rollback':'head','malformed':'version','whitespace':'format'}.get(kind,'path')
            step('stored-'+kind,dict(op='read',handle='a',head=head),expected,unchanged=True)
            for p,raw in zip(roots,saved):
                path=p/'main/config.json'
                if kind=='hardlink':(p/'main/hard').unlink()
                else:
                    path.unlink();path.write_bytes(raw);path.chmod(0o600)
            read(head)
        step('close-first-handle',dict(op='close',handle='a'));step('closed-handle-read',dict(op='read',handle='a',head=head),'closed',unchanged=True)
        step('reopen',dict(op='open',dir='main',handle='reopened'));assert step('reopen-read',dict(op='read',handle='reopened',head=head))['view']==second
        for depth,expected in [(1,'version'),(10000,'version'),(10001,'format')]:
            raw=b'{"schema_version":2,"unknown":'+b'['*(depth-1)+b'0'+b']'*(depth-1)+b'}'
            step('json-depth-control',dict(op='decode',raw=b'{"schema_version":2,"unknown":true}'.hex()),'version',unchanged=True)
            step('json-depth-'+str(depth),dict(op='decode',raw=raw.hex()),expected,unchanged=True)
        for c in clients:c.close()
        clients=[]
        # Reopen the other language's exact files with independent expected heads.
        for source,root,reader,writer in [('go-to-rust',roots[0],rust,go),('rust-to-go',roots[1],go,rust)]:
            first_client=Client(reader,root,output,source+'-reader',env);clients.append(first_client)
            assert first_client.call(dict(op='open',dir='main',handle='cross'))['code']=='ok'
            assert first_client.call(dict(op='read',handle='cross',head=head))['view']==second
            c=change('cross-language',head);cmd=dict(op='apply',handle='cross',change=c,at=at);reply=first_client.call(cmd);assert reply['code']=='ok'and reply['view']['head']['revision']=='3'
            new=reply['view'];first_client.close();clients.remove(first_client);before=(root/'main/config.json').read_bytes()
            other=Client(writer,root,output,source+'-writer',env);clients.append(other);assert other.call(dict(op='open',dir='main',handle='cross'))['code']=='ok'
            assert other.call(dict(op='read',handle='cross',head=new['head']))['view']==new
            assert other.call(cmd)['view']==new and(root/'main/config.json').read_bytes()==before
            assert other.call(dict(op='read',handle='cross',head=head))['code']=='head'
            other.close();clients.remove(other);cross.append(dict(direction=source,read=True,write=True,replay_without_mutation=True,head=new['head']))
        return dict(status='PASS',paired_operations=len(cases),names=cases,cross_language=cross,fixture_roots=[str(p)for p in roots])
    finally:
        for c in clients:c.abort()
