"""Compare the actual coordinated CLI, including canonical witness refusals.
Only isolated directories are written. No timers or provider/service operations.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
from store_scenarios import Client

def run(go,rust,work,output,env):
    roots=[work/'go-coordinator',work/'rust-coordinator']
    for p in roots:p.mkdir(mode=0o700)
    clients=[Client(binary,root,output,label,env)for binary,root,label in zip([go,rust],roots,['coordinator-go','coordinator-rust'])]
    cases=[];cross=[]
    zero=dict(revision='0',digest='');at='2026-10-17T12:00:00Z'
    def change(event,head):return dict(schema_version=1,event_id=event,expected_head=head,values=dict(mode='local',local_socket_path='/run/swarm-fixture/authority.sock'),reason='explicit fixture choice')
    def call(op,c=None,**extra):return dict(op=op,handle='a',**(dict(change=c,at=at)if c else{}),**extra)
    def documents():
        result=[]
        for root in roots:
            record={}
            for name in ['config/config.json','witness/witness.json']:
                p=root/name
                if not p.exists()and not p.is_symlink():record[name]=None;continue
                st=p.lstat();record[name]=dict(mode=stat.S_IMODE(st.st_mode),links=st.st_nlink,type=stat.S_IFMT(st.st_mode),digest=hashlib.sha256(p.read_bytes()).hexdigest()if stat.S_ISREG(st.st_mode)else None,link=os.readlink(p)if p.is_symlink()else None)
            result.append(record)
        return result
    def step(name,command,expected='ok',unchanged=False):
        before=documents()if unchanged else None
        result=[c.call(command)for c in clients];assert result[0]==result[1],name+': Go/Rust divergence'
        assert result[0]['code']==expected,(name,result[0]['code'],expected)
        if unchanged:assert documents()==before,name+': unexpected mutation'
        cases.append(name)
        if len(cases)%20==0:print('coordinator scenarios',len(cases),flush=True)
        return result[0]
    def read(expected=None):
        v=step('valid-coordinated-read-control',call('read'))['view']
        if expected:assert v==expected
        return v
    def raw(name):
        data=[(p/name).read_bytes()for p in roots];assert data[0]==data[1];return data[0]
    def write(name,data):
        for p in roots:
            target=p/name;target.write_bytes(data);target.chmod(0o600)
    def compact(v):return json.dumps(v,separators=(',',':'),ensure_ascii=False).encode()
    try:
        for name in ['config','witness']:step('mkdir-'+name,dict(op='mkdir',dir=name))
        step('open-coordinated',dict(op='open',dir='config',witness_dir='witness',handle='a',coordinated=True))
        step('empty-read',call('read'),'not_configured',True);step('empty-recover',call('recover'),'not_configured',True)
        initial=change('initial',zero);p=step('preview-without-documents',call('preview',initial),unchanged=True)['projection']
        first=step('create-both-documents',call('create',initial))['view'];assert first==p['proposed'];read(first)
        firstc=raw('config/config.json');firstw=raw('witness/witness.json')
        next=change('next',first['head']);p=step('preview-next',call('preview',next),unchanged=True)['projection']
        second=step('apply-both-documents',call('apply',next))['view'];read(second)
        secondc=raw('config/config.json');secondw=raw('witness/witness.json')
        step('exact-replay',dict(call('apply',next),at='2026-10-18T12:00:00Z'),unchanged=True)
        for name,command,expected in [('stale',call('apply',change('stale',first['head'])),'conflict'),('old-event',call('apply',initial),'event'),('canceled-read',call('read',canceled=True),'canceled'),('deadline-recover',call('recover',deadline=True),'deadline')]:
            read(second);step(name,command,expected,True);read(second)
        for name,old in [('config/config.json',firstc),('witness/witness.json',firstw)]:
            read(second);write(name,old);step('single-store-rollback-'+name,call('read'),'head',True);step('rollback-recover-refused-'+name,call('recover'),'head',True)
            write('config/config.json',secondc);write('witness/witness.json',secondw);read(second)
        write('config/config.json',firstc);write('witness/witness.json',firstw);read(first) # stated local limitation
        for p in roots:(p/'witness/witness.json').unlink()
        step('missing-witness',call('read'),'witness_missing',True);step('never-infer-missing-witness',call('create',initial),'witness_missing',True)
        write('witness/witness.json',firstw);read(first)
        # Codec variants run through the public reader; every fault has a valid control.
        def codec(name,data,expected):
            read(first);write('witness/witness.json',data);step('witness-'+name,call('read'),expected,True);write('witness/witness.json',firstw);read(first)
        variants=[('negative-zero',firstw.replace(b'\"schema_version\":1',b'\"schema_version\":-0',1),'version'),('decimal-negative-zero',firstw.replace(b'\"schema_version\":1',b'\"schema_version\":-0.0',1),'witness'),('whitespace',b' '+firstw,'witness'),('future',firstw.replace(b'"schema_version":1',b'"schema_version":2',1),'version'),('empty',b'{}','version'),('null',b'null','version'),('unknown',b'{"unknown":true,'+firstw[1:],'witness'),('duplicate',b'{"schema_version":1,'+firstw[1:],'witness'),('folded',firstw.replace(b'schema_version',b'Schema_Version',1),'witness'),('null-version',firstw.replace(b'"schema_version":1',b'"schema_version":null',1),'version'),('numeric-revision',firstw.replace(b'"revision":"1"',b'"revision":1',1),'witness'),('trailing',firstw+b'{}','witness'),('future-wrong-type',b'{"schema_version":2,"pins":false}','witness'),('invalid-utf8-string',b'{"schema_version":2,"unknown":"\xff"}','version'),('nan',b'{"unknown":NaN}','witness')]
        for name,data,expected in variants:codec(name,data,expected)
        for depth in [1,9999,10000]:codec('depth-'+str(depth),b'{"schema_version":2,"unknown":'+b'['*depth+b'0'+b']'*depth+b'}','version'if depth<10000 else'witness')
        for field,value,expected in [('schema_version',False,'witness'),('schema_version','1','witness'),('pins',None,'witness'),('committed',None,'witness'),('pending',{},'version')]:
            v=json.loads(firstw);v[field]=value;codec('type-'+field+'-'+str(value),compact(v),expected)
        other=firstw.replace(b'a'*64,b'b'*64,1);codec('valid-other-pins',other,'pins')
        for mode in [0o640,0o644]:
            read(first)
            for p in roots:(p/'witness/witness.json').chmod(mode)
            step('unsafe-witness-mode-'+oct(mode),call('read'),'path',True)
            for p in roots:(p/'witness/witness.json').chmod(0o600)
            read(first)
        for kind in ['hardlink','symlink','fifo']:
            read(first)
            for root in roots:
                path=root/'witness/witness.json';other=root/'witness/other'
                if kind=='hardlink':os.link(path,other)
                elif kind=='symlink':path.rename(other);path.symlink_to(other)
                else:path.unlink();os.mkfifo(path,0o600)
            step('unsafe-witness-'+kind,call('read'),'path',True)
            for root in roots:
                path=root/'witness/witness.json';other=root/'witness/other'
                if kind=='hardlink':other.unlink()
                elif kind=='symlink':path.unlink();other.rename(path)
                else:path.unlink()
            write('witness/witness.json',firstw);read(first)
        # Reconstruct an authorized prepared decision using actual Go/Rust preview.
        preview=step('pending-intent-valid-control',call('preview',next),unchanged=True)['projection']
        pending=dict(schema_version=1,pins=first['snapshot']['pins'],committed=first['head'],pending=dict(entry=preview['proposed']['snapshot']['history'][-1],target=preview['proposed']['head']))
        intent=compact(pending)
        for phase in ['before-config','already-target']:
            write('config/config.json',firstc if phase=='before-config'else secondc);write('witness/witness.json',intent)
            step('pending-read-'+phase,call('read'),'pending',True);step('pending-preview-'+phase,call('preview',next),'pending',True)
            step('different-request-'+phase,call('apply',change('unrelated',first['head'])),'pending',True)
            for mode in ['deny','wrong','cancel']:
                handle='recovery-'+phase+'-'+mode;step('open-'+handle,dict(op='open',dir='config',witness_dir='witness',handle=handle,coordinated=True,authority=mode))
                step('denied-'+handle,dict(op='recover',handle=handle),'canceled'if mode=='cancel'else'unauthorized',True)
            v=step('authorized-recover-'+phase,call('recover'))['view'];assert v==second;read(second)
            assert raw('config/config.json')==secondc and raw('witness/witness.json')==secondw
        # Pending initial decision can also be adopted after reopening, but never
        # guessed from a configuration whose witness is missing.
        initial_intent=dict(schema_version=1,pins=first['snapshot']['pins'],committed=zero,pending=dict(entry=first['snapshot']['history'][0],target=first['head']))
        for root in roots:(root/'config/config.json').unlink()
        write('witness/witness.json',compact(initial_intent));step('pending-bootstrap-read',call('read'),'pending',True)
        step('bootstrap-recover',call('recover'));read(first)
        # Cross-language actual files, independently opened, read/write/replay.
        for index,label in [(0,'go-to-rust'),(1,'rust-to-go')]:
            root=roots[index];binary=rust if index==0 else go
            peer=Client(binary,root,output,'coordinator-cross-'+label,env)
            try:
                assert peer.call(dict(op='open',dir='config',witness_dir='witness',handle='peer',coordinated=True))['code']=='ok'
                assert peer.call(dict(op='read',handle='peer'))['view']==first
                cc=change('cross-'+label,first['head']);v=peer.call(dict(op='apply',handle='peer',change=cc,at=at))['view'];assert v['head']['revision']=='2'
                assert clients[index].call(call('read'))['view']==v
                saved=(root/'config/config.json').read_bytes(),(root/'witness/witness.json').read_bytes()
                assert clients[index].call(call('apply',cc))['view']==v
                assert saved==((root/'config/config.json').read_bytes(),(root/'witness/witness.json').read_bytes())
                cross.append(dict(direction=label,head=v['head'],read_write_replay=True))
            finally:peer.close()
        for c in clients:c.close()
        # Process exits after the actual native intent fsync, without Close or
        # destructor cleanup. A fresh Go process must reauthorize and recover.
        crashroot=work/'native-interrupted';crashroot.mkdir(mode=0o700)
        crashed=Client(rust,crashroot,output,'coordinator-interrupted-native',env)
        try:
            for name in ['config','witness']:assert crashed.call(dict(op='mkdir',dir=name))['code']=='ok'
            assert crashed.call(dict(op='open',dir='config',witness_dir='witness',handle='a',coordinated=True))['code']=='ok'
            assert crashed.call(call('preview',initial))['projection']['proposed']==first
            assert crashed.call(call('interrupt-intent'))['code']=='ok'
            cmd=call('create',initial);crashed.log.write(json.dumps(dict(input=cmd,expected_process_exit_after_intent=True))+'\n');crashed.log.flush()
            crashed.process.stdin.write(compact(cmd)+b'\n');crashed.process.stdin.flush()
            assert crashed.process.stdout.readline()==b'';assert crashed.process.wait()==0
            crashed.close()
        except BaseException:crashed.abort();raise
        assert not(crashroot/'config/config.json').exists()and(crashroot/'witness/witness.json').exists()
        resumed=Client(go,crashroot,output,'coordinator-interrupted-go-recovery',env)
        try:
            assert resumed.call(dict(op='open',dir='config',witness_dir='witness',handle='a',coordinated=True))['code']=='ok'
            assert resumed.call(call('read'))['code']=='pending'
            assert resumed.call(call('recover'))['view']==first
            assert resumed.call(call('read'))['view']==first
        finally:resumed.close()
        return dict(interrupted_native_intent_recovered_by_go=True,paired_operations=len(cases),cases=cases,cross_language=cross,witness_only_configuration_only_rollback_refused=True,restoring_both_stores_explicit_limitation=True)
    except BaseException:
        for c in clients:c.abort()
        raise
