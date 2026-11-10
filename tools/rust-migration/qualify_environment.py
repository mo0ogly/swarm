"""Controlled Git command boundary, including failures retained per field."""
import hashlib,json,os,sys
from pathlib import Path
def qualify(output,work,env,run,save,reference,probe):
    binary=work/'git-fixture-bin';binary.mkdir();script=binary/'git'
    source='#!'+sys.executable+'''\nimport json,os,sys
from pathlib import Path
spec=json.loads(Path(os.environ['SWARM_GIT_FIXTURE']).read_text())
with open(spec['trace'],'a')as stream:stream.write(json.dumps(sys.argv[1:])+'\\n')
assert sys.argv[1:3]==['-C',spec['root']]
value=spec['commands'][' '.join(sys.argv[3:])]
sys.stdout.buffer.write(bytes.fromhex(value['stdout']));sys.stdout.buffer.flush()
sys.stderr.buffer.write(b'PRIVATE controlled Git diagnostic');sys.stderr.buffer.flush()
raise SystemExit(value['exit'])
'''
    script.write_text(source);script.chmod(0o700);interpreter=Path(sys.executable).resolve();binding=dict(script_sha256=hashlib.sha256(script.read_bytes()).hexdigest(),interpreter=str(interpreter),interpreter_sha256=hashlib.sha256(interpreter.read_bytes()).hexdigest())
    manifest=work/'git-fixture.json';trace=work/'git-trace.jsonl';root=work/'git-root';root.mkdir();keys=['rev-parse HEAD','branch --show-current','status --porcelain --untracked-files=normal'];cases=[]
    prior={k:env.get(k)for k in ['PATH','SWARM_GIT_FIXTURE']}
    try:
        env['PATH']=str(binary);env['SWARM_GIT_FIXTURE']=str(manifest)
        for index,exits in enumerate([[0,0,0],[128,0,0],[0,1,0],[0,0,1],[128,128,128]]):
            commands={key:dict(stdout=raw.hex(),exit=code)for key,raw,code in zip(keys,[b' head\n',b' branch\n',b'\xe2\x82\n'],exits)}
            spec=dict(root=str(root),trace=str(trace),commands=commands);manifest.write_text(json.dumps(spec));frame=dict(Name='git-'+str(index),Mode='git',Bytes='',Prefix='',Seconds=0,Nanos=0,Root=str(root));data=json.dumps(frame).encode()+b'\n';pair=[]
            for language,command in [('go',reference),('rust',probe)]:
                trace.write_text('');value=json.loads(run('git-'+language+'-'+str(index),[str(command)],data));calls=[json.loads(l)for l in trace.read_text().splitlines()];assert calls==[['-C',str(root),*key.split()]for key in keys];pair.append(value)
            go,native=pair;assert go['name']==native['name']and json.loads(bytes.fromhex(go['value']))==json.loads(bytes.fromhex(native['value']));assert native['issues']==sum(code!=0 for code in exits)
            cases.append(dict(input=frame,command_results=commands,go=go,rust=native,actual_three_calls_each=True))
        empty=work/'no-git';empty.mkdir();env['PATH']=str(empty);frame=dict(Name='git-spawn-refused',Mode='git',Bytes='',Prefix='',Seconds=0,Nanos=0,Root=str(root));data=json.dumps(frame).encode()+b'\n';go=json.loads(run('git-go-spawn-refused',[str(reference)],data));native=json.loads(run('git-rust-spawn-refused',[str(probe)],data));assert go=={k:v for k,v in native.items()if k!='issues'}and native['issues']==3;cases.append(dict(input=frame,go=go,rust=native))
    finally:
        for key,value in prior.items():
            if value is None:env.pop(key,None)
            else:env[key]=value
    assert hashlib.sha256(script.read_bytes()).hexdigest()==binding['script_sha256']and hashlib.sha256(interpreter.read_bytes()).hexdigest()==binding['interpreter_sha256'];report=dict(status='PASS',cases=cases,binding=binding,scope='Owned fake Git plus actual command argument/exit/output boundaries; not acceptance freshness')
    save('git-observation-cases.json',report);return report
