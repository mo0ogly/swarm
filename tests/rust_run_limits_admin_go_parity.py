#!/usr/bin/env python3
"""Actual Go/native run-limit CLI differential on independent private roots.

Go source reference is retained separately. Only common old-policy values are
compared; native saved ceilings and preview token are extensions, not inferred
Go parity. Actor/time are local observations deliberately excluded from equality.
"""
import argparse,json,os,subprocess,tempfile,traceback
from pathlib import Path
from rust_agent_model_route import publish,digest

def run(binary,go,output):
    """Retain raw replies and require matching current/history/replay/rollback.
    No provider, live mission, global deadline or mutation outside owned roots.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());commands=[];roots={};checks=[]
    for kind in ['native','go']:
        roots[kind]=Path(tempfile.mkdtemp(prefix='limits-parity-'+kind+'-'));(roots[kind]/'empty-bin').mkdir()
    def call(kind,args,value=None,success=True):
        number=len(commands);exe=binary if kind=='native' else go;argv=[str(exe),*args,'--root',str(roots[kind]),'--json']
        if value is not None:
            p=output/(str(number)+'.input.json');publish(p,value);argv+=['--input',str(p)]
        r=subprocess.run(argv,env={'PATH':str(roots[kind]/'empty-bin'),'USER':'fixture','LANG':'C.UTF-8'},stdin=subprocess.DEVNULL,capture_output=True)
        publish(output/(str(number)+'.stdout'),r.stdout);publish(output/(str(number)+'.stderr'),r.stderr);commands.append(dict(argv=argv,exit=r.returncode));publish(output/'commands.json',commands)
        assert (r.returncode==0)==success,(kind,args,r.returncode,r.stderr)
        return json.loads(r.stdout) if success else None
    def projection(v):
        if isinstance(v,list):return [projection(row) for row in v]
        if isinstance(v,dict):return {k:projection(value) for k,value in v.items() if k not in ['actor','updated','at']}
        return v
    def both(args,value=None,success=True):
        a=call('go',args,value,success);b=call('native',args,value,success)
        if success:assert projection(a)==projection(b),(args,projection(a),projection(b))
        return b
    for kind in roots:call(kind,['init'])
    both(['run-limits','show','project','-','-']);assert both(['run-limits','history','project','-','-']) is None;checks.append('empty-current-and-null-history')
    r=dict(schema_version=1,event_id='one',scope='project',expected_revision=0,values=dict(silence_seconds=2,tool_seconds=3,max_tool_calls=4,max_repeated_calls=2,max_consecutive_errors=2),reason='Actual Go/native fixture')
    assert both(['run-limits','apply','project','-','-'],r)['revision']==1;checks.append('explicit-cas-apply')
    second={**r,'event_id':'two','expected_revision':1,'values':{**r['values'],'silence_seconds':7}}
    assert both(['run-limits','apply','project','-','-'],second)['revision']==2
    assert both(['run-limits','apply','project','-','-'],r)['revision']==2;assert len(both(['run-limits','history','project','-','-']))==2;checks.append('replay-returns-current-without-append')
    both(['run-limits','apply','project','-','-'],{**r,'event_id':'stale'},False);both(['run-limits','apply','project','-','-'],{**r,'reason':'changed'},False);checks.append('stale-and-conflicting-event-refuse')
    rollback=dict(event_id='restore',expected_revision=2,reason='')
    row=both(['run-limits','rollback','project','-','-','1'],rollback);assert row['revision']==3 and row['values']['silence_seconds']==2
    history=both(['run-limits','history','project','-','-']);assert [v['revision'] for v in history]==[3,2,1] and history[0]['rollback_of']==1;checks.append('rollback-appends-history-and-go-default-reason')
    for sc,mission,key in [('mission','m',''),('role','m','worker'),('task','m','task')]:
        request={**r,'event_id':'scope-'+sc,'scope':sc,'mission_id':mission,'scope_key':key}
        assert both(['run-limits','apply',sc,mission,key or '-'],request)['revision']==1
        both(['run-limits','show',sc,mission,key or '-']);both(['run-limits','history',sc,mission,key or '-'])
    checks.append('mission-role-task-tuples')
    publish(output/'result.json',dict(status='PASS',checks=checks,native_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),roots={k:str(v) for k,v in roots.items()},scope='common CLI contract; not global equivalence or live installation'))
    print('RUN_LIMITS_GO_PARITY_VERIFIED',len(checks),flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
