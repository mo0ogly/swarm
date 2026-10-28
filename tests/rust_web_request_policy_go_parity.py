#!/usr/bin/env python3
"""Go/native web-request file differential: actual CLI, private owned roots.

Common format and replay semantics only; native saved caps/optional token and
nonempty reason deliberately differ from old Go hardcoded policies. No provider,
live mission, global watchdog or accepted identity is inferred.
"""
from pathlib import Path
import argparse,json,os,subprocess,tempfile,traceback
from rust_agent_model_route import publish,digest

def run(binary,go,output):
    """Retain raw operations and valid controls before each fixture family.

    Private source replacements test wire cases, never reset consumption or a
    live history. Projections exclude only observed actor/time and native view
    extensions; actual values/config/history remain compared.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());commands=[];roots={kind:Path(tempfile.mkdtemp(prefix='web-policy-parity-'+kind+'-')) for kind in ['go','native']};checks=[]
    for root in roots.values():(root/'empty-bin').mkdir()
    def call(kind,args,value=None,success=True):
        exe=go if kind=='go' else binary;argv=[str(exe),*args,'--root',str(roots[kind]),'--json'];number=len(commands)
        if value is not None:
            p=output/(str(number)+'.input.json');publish(p,value);argv+=['--input',str(p)]
        r=subprocess.run(argv,env={'PATH':str(roots[kind]/'empty-bin'),'USER':'fixture','LANG':'C.UTF-8'},stdin=subprocess.DEVNULL,capture_output=True)
        publish(output/(str(number)+'.stdout'),r.stdout);publish(output/(str(number)+'.stderr'),r.stderr);commands.append(dict(kind=kind,argv=argv,exit=r.returncode));publish(output/'commands.json',commands)
        assert (r.returncode==0)==success,(kind,args,r.returncode)
        return json.loads(r.stdout) if success else None
    def projection(v):
        if isinstance(v,list):return [projection(row) for row in v]
        if isinstance(v,dict):return {k:projection(value) for k,value in v.items() if k not in ['actor','at']}
        return v
    def both(op,value=None,success=True):
        replies=[call(kind,['run-limits','web-requests',op],value,success) for kind in ['go','native']]
        if not success:return None
        if op=='show':replies=[{k:r[k] for k in ['schema_version','revision','values','history']} for r in replies]
        if op in ['apply','preview']:replies=[{k:r[k] for k in ['applied','config']} for r in replies]
        assert projection(replies[0])==projection(replies[1]),(op,projection(replies[0]),projection(replies[1]));return replies[1]
    for kind in roots:call(kind,['init'])
    assert both('show')['revision']==0 and both('history')==[];checks.append('versioned-defaults-and-empty-history')
    r=dict(schema_version=1,event_id='first',expected_revision=0,values=dict(general_ms=0,snapshot_ms=30,connection_test_ms=40),reason='Actual Go comparison')
    assert both('preview',r)['config']['revision']==1 and both('show')['revision']==0;checks.append('preview-without-publication')
    assert both('apply',r)['config']['revision']==1
    second={**r,'event_id':'second','expected_revision':1,'values':{**r['values'],'general_ms':2}}
    assert both('apply',second)['config']['revision']==2;assert both('apply',r)['config']['revision']==2 and len(both('history'))==2;checks.append('explicit-cas-and-replay-current-history')
    both('apply',{**r,'event_id':'stale'},False);both('apply',{**r,'reason':'Different intent'},False);assert both('show')['revision']==2;checks.append('stale-and-event-conflict-no-change')
    originals={kind:(root/'.swarm/web-requests.json').read_bytes() for kind,root in roots.items()}
    zero=dict(general_ms=0,snapshot_ms=0,connection_test_ms=0);base=dict(schema_version=1,revision=0,values=zero,history=[])
    def wire(value):return json.dumps(value,separators=(',',':')).encode()
    prefix=b'{"schema_version":1,"revision":0,"history":[],'
    cases=[('valid-control',wire(base),True),('missing-values',b'{"schema_version":1,"revision":0,"history":null}',True),('present-null',wire({**base,'values':None}),False),('partial',wire({**base,'values':{'general_ms':0}}),False),('earlier-partial',prefix+b'"values":{"general_ms":0},"values":{"general_ms":0,"snapshot_ms":0,"connection_test_ms":0}}',False),('earlier-null',prefix+b'"values":null,"values":{"general_ms":0,"snapshot_ms":0,"connection_test_ms":0}}',False),('full-duplicates',prefix+b'"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"values":{"general_ms":4,"snapshot_ms":5,"connection_test_ms":6}}',True),('last-field-null',prefix+b'"values":{"general_ms":1,"general_ms":null,"snapshot_ms":0,"connection_test_ms":0}}',False),('null-then-int',prefix+b'"values":{"general_ms":null,"general_ms":2,"snapshot_ms":0,"connection_test_ms":0}}',True),('bad-then-int',prefix+b'"values":{"general_ms":"bad","general_ms":2,"snapshot_ms":0,"connection_test_ms":0}}',False),('casefold',b'{"schema_version":1,"revision":0,"VALUES":{"General_MS":10,"general_ms":15,"Snapshot_ms":60,"CONNECTION_TEST_MS":65}}',True),('safe-max',wire({**base,'values':{**zero,'general_ms':9007199254740991}}),True),('unsafe-int',wire({**base,'values':{**zero,'general_ms':9007199254740992}}),False)]
    for name,raw,success in cases:
        publish(output/(name+'.source.json'),raw)
        for kind,root in roots.items():publish(root/'.swarm/web-requests.json',raw)
        both('show',success=success)
    checks.append('thirteen-raw-wire-cases-required-null-duplicates-casefold-exact-integer')
    history=dict(schema_version=1,revision=1,values=zero,history=[dict(revision=1,values=zero,actor='',at='',change=None)])
    for name,value,success in [('null-change',history,True),('invalid-header-metadata',{**history,'history':[{**history['history'][0],'change':dict(schema_version=9,event_id='',expected_revision=-1,values=zero,reason='')}]},True),('change-present-null-values',{**history,'history':[{**history['history'][0],'change':dict(values=None)}]},False)]:
        raw=wire(value);publish(output/(name+'.source.json'),raw)
        for kind,root in roots.items():publish(root/'.swarm/web-requests.json',raw)
        both('show',success=success)
    for kind,root in roots.items():publish(root/'.swarm/web-requests.json',originals[kind])
    assert both('show')['revision']==2;checks.append('three-historical-metadata-cases-and-original-recovery')
    publish(output/'result.json',dict(status='PASS',checks=checks,raw_wire_cases=16,native_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),roots={k:str(v) for k,v in roots.items()},scope='bounded public differential; not global equivalence or provider acceptance'))
    print('WEB_REQUEST_POLICY_GO_PARITY_VERIFIED',len(checks),'WIRE16',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
