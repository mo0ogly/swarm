"""Create owned filesystem fixtures and vectors; no active mission paths."""
import json,os
from pathlib import Path

def vectors(work):
    base=work/'path-fixture';base.mkdir(mode=0o700);root=base/'root';outside=base/'outside';root.mkdir();outside.mkdir();(root/'dir').mkdir();(outside/'nested').mkdir()
    (root/'file').write_bytes(b'content');(root/'é.txt').write_bytes(b'unicode');(root/' spaced ').write_bytes(b'spaces');(outside/'file').write_bytes(b'outside')
    (base/'alias').symlink_to(root,target_is_directory=True);(root/'in-link').symlink_to('file');(root/'out-link').symlink_to(outside/'file');(root/'dir-link').symlink_to(outside/'nested',target_is_directory=True);(root/'inner-root').symlink_to(root,target_is_directory=True);(root/'broken').symlink_to('absent');(root/'loop').symlink_to('loop');os.mkfifo(root/'fifo')
    nonutf=os.fsencode(base)+b'/nonutf-\xff';os.mkdir(nonutf);fd=os.open(nonutf+b'/file',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600);os.write(fd,b'opaque');os.close(fd)
    cases=[]
    def frame(name,mode='local',r=root,path='',file='file',body=None,request=None,control=False,classification='parity'):
        w=body or dict(schema_version=1,id='work',revision=2,summary='old',next='old next',memory=['old'],tasks=[])
        return dict(Name=name,Mode=mode,RootHex=os.fsencode(r).hex(),PathHex=os.fsencode(path).hex(),File=file,Body=json.dumps(w,separators=(',',':')).encode().hex(),Request=json.dumps(request or dict(schema_version=1,summary='New checkpoint',next='Next',memory=['file']),separators=(',',':')).encode().hex(),Git=dict(head='head',branch='branch',changes='changes'),control=control,classification=classification)
    def pair(name,**kw):cases.append(frame(name+'-control',control=True));cases.append(frame(name,**kw))
    for name in ['file','./file','dir/../file','in-link','é.txt',' spaced ','',' ','\u0085\u3000','/dev/null','../outside/file','out-link','dir','fifo','broken','loop','missing','dir-link/../file','dir-link/../../outside/file','file/../file','././file','dir//../file']:
        pair('local-'+str(len(cases)),file=name)
    for r in [root,base/'alias',str(base/'alias')+'/..',str(root)+'/dir-link/..',str(root)+'/dir/../',base/'absent',root/'file',nonutf,os.path.relpath(root),Path('/')]:
        cases.append(frame('root-'+str(len(cases)),mode='root',r=r))
    for r in [root,base/'alias',nonutf]:
        for suffix in ['', '/file','/dir','/in-link','/inner-root/file','/dir-link','/dir-link/../file','/file/','//file','/./file','/missing']:
            prefix=os.fsdecode(r);path=prefix+suffix
            if isinstance(r,bytes):path=os.fsencode(r)+os.fsencode(suffix)
            pair('owned-'+str(len(cases)),mode='owned',r=r,path=path)
    for path in ['relative',outside/'file',base/'alias/file',root/'inner-root/file']:
        pair('owned-explicit-'+str(len(cases)),mode='owned',path=path)
    for count in [1,40,41,64,255,256]:
        for i in reversed(range(count)):
            (root/('chain-'+str(count)+'-'+str(i))).symlink_to('file'if i+1==count else 'chain-'+str(count)+'-'+str(i+1))
            (root/('root-chain-'+str(count)+'-'+str(i))).symlink_to('dir'if i+1==count else 'root-chain-'+str(count)+'-'+str(i+1))
        pair('chain-local-'+str(count),file='chain-'+str(count)+'-0')
        cases.append(frame('chain-root-'+str(count),mode='root',r=root/('root-chain-'+str(count)+'-0')))
        cases.append(frame('chain-owned-'+str(count),mode='owned',r=root/'dir',path=root/('root-chain-'+str(count)+'-0')))
    for name,target in [('target-slash','file/'),('target-dot','file/.'),('target-parent-file','file/../file'),('target-parent-dir','dir/../file')]:
        (root/name).symlink_to(target);pair(name,file=name)
    for memory in [None,[],['file'],['in-link'],['dir-link/../file'],['file','é.txt'],['file','missing'],['file','../outside/file'],['fifo'],['dir'],['broken'],['loop'],[' ']]:
        request=dict(schema_version=1,summary='New checkpoint',next='Next',memory=memory)
        bad=memory and any(v in ['missing','../outside/file','fifo','dir','broken','loop',' ']for v in memory)
        cases.append(frame('checkpoint-'+str(len(cases)),mode='checkpoint',request=request,classification='atomicity'if bad else 'parity',control=not bad))
    for summary in ['', ' ','\u0085','\ufeff','Summary']:
        cases.append(frame('summary-'+str(len(cases)),mode='checkpoint',request=dict(schema_version=1,summary=summary,memory=[])))
    cases.append(frame('schema-invalid',mode='checkpoint',request=dict(schema_version=0,summary='Summary',memory=[])))
    assert len(cases)==len({v['Name']for v in cases});return cases
