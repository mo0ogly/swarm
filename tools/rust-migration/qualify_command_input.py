"""Real public Go/native CLI framing checks on separate owned roots.
This compares document framing, not equality of different command DTOs.
"""
import hashlib,json,os,sqlite3,subprocess,tempfile
from pathlib import Path
LIMIT=16*1024*1024
def qualify(root,output,native,go):
    output.mkdir();scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-framing-',dir=Path.home().resolve()));commands=[]
    for path in [scratch,*scratch.parents]:s=path.stat();assert s.st_uid in [0,os.geteuid()]and not s.st_mode&0o22
    before={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in [native,go]}
    def execute(label,binary,project,args,data=None,expected=0):
        argv=[str(binary),'--root',str(project),'--json',*args]
        p=subprocess.run(argv,input=data,capture_output=True,cwd=root)
        (output/(label+'.stdout')).write_bytes(p.stdout);(output/(label+'.stderr')).write_bytes(p.stderr)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        commands.append(dict(label=label,args=argv,exit=p.returncode,expected=expected));(output/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print('frame',label,'exit',p.returncode,flush=True)
        assert p.returncode==expected,label+': original failure retained';return p
    projects={name:scratch/name for name in ['go','native']}
    for name,binary in [('go',go),('native',native)]:projects[name].mkdir(mode=0o700);execute(name+'-init',binary,projects[name],['init'])
    g=projects['go'];n=projects['native'];doc=json.loads(execute('native-show',native,n,['engine-settings','show']).stdout)
    native_request=dict(schema_version=doc['schema_version'],event_id='framing-control',expected_revision=doc['revision'],values=doc['values'],reason='Document framing control')
    go_request=dict(schema_version=1,event_id='framing-control',expected_revision=0,title='Framing control',objective='Observe document boundary',scope='Isolated transport fixture',criteria=['No extra mutation'])
    native_file=n/'request.json';go_file=g/'request.json';raws={native_file:json.dumps(native_request).encode(),go_file:json.dumps(go_request).encode()}
    def write(file,data):file.write_bytes(data);file.chmod(0o600)
    for file,data in raws.items():write(file,data)
    def invoke(label,name,expected=0,file=None,data=None):
        binary=go if name=='go'else native;project=projects[name]
        args=['work','create']if name=='go'else['engine-settings','preview']
        return execute(label,binary,project,[*args,'--input',str(file if file is not None else('-'if data is not None else project/'request.json'))],data,expected)
    expected_work=None;expected_preview=None;settings_before=(n/'.swarm/engine-settings.json').read_bytes()
    for name,file in [('go',go_file),('native',native_file)]:
        control=json.loads(invoke(name+'-valid-control',name).stdout)
        if name=='go':expected_work=control['work'];assert len(control['events'])==1
        else:expected_preview=control['preview_token']
        write(file,raws[file]+b' '*(LIMIT-len(raws[file])))
        boundary=json.loads(invoke(name+'-exact-boundary',name).stdout)
        if name=='go':assert boundary['work']==expected_work and len(boundary['events'])==1
        else:assert boundary['preview_token']==expected_preview
        write(file,raws[file]);invoke(name+'-pre-overflow-control',name)
        write(file,raws[file]+b' '*(LIMIT+1-len(raws[file])))
        overflow=invoke(name+'-overflow-refused',name,2)
        if name=='go':assert 'entrée supérieure à 16 Mio'.encode()in overflow.stderr
        else:assert json.loads(overflow.stderr)['failure']['code']=='input-document-too-large'
        write(file,raws[file]);recovered=json.loads(invoke(name+'-recovery',name).stdout)
        if name=='go':assert recovered['work']==expected_work and len(recovered['events'])==1
        else:assert recovered['preview_token']==expected_preview
        stdin=json.loads(invoke(name+'-finite-stdin',name,data=raws[file]).stdout)
        if name=='go':assert stdin['work']==expected_work and len(stdin['events'])==1
        else:assert stdin['preview_token']==expected_preview
    fifo=n/'request.fifo';os.mkfifo(fifo,0o600)
    for label,file in [('fifo',fifo),('device',Path('/dev/zero')),('directory',n)]:
        invoke(label+'-valid-control','native');refused=json.loads(invoke(label+'-refused','native',2,file).stderr);assert refused['failure']['code']=='input-regular-file-required'
        invoke(label+'-recovery','native');assert(n/'.swarm/engine-settings.json').read_bytes()==settings_before
    alias=n/'alias.json';alias.symlink_to(native_file);assert json.loads(invoke('native-regular-alias','native',file=alias).stdout)['preview_token']==expected_preview
    with sqlite3.connect(g/'.swarm/state.db')as db:assert db.execute('SELECT count(*)FROM works').fetchone()[0]==db.execute('SELECT count(*)FROM events').fetchone()[0]==1
    assert(n/'.swarm/engine-settings.json').read_bytes()==settings_before
    assert before=={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in [native,go]}
    result=dict(status='PASS',scope='Byte framing of real public Go work.create vs native settings.preview, separate valid DTOs; no full Work CLI parity claim',limit_bytes=LIMIT,exact_boundary_both=True,overflow_refused_both=True,finite_stdin_both=True,native_named_fifo_device_directory_refused=True,native_regular_alias_supported=True,no_extra_work_event_or_settings_mutation=True,binaries=before,scratch=str(scratch),global_watchdog=None)
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');return result
