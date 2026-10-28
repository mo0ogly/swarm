#!/usr/bin/env python3
"""Go/native graph objectives file differential: actual CLI, private owned roots.

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
        replies=[call(kind,['run-limits','performance',op],value,success) for kind in ['go','native']]
        if not success:return None
        if op=='show':replies=[{k:r[k] for k in ['schema_version','revision','values','history']} for r in replies]
        if op in ['apply','preview']:replies=[{k:r[k] for k in ['applied','config']} for r in replies]
        assert projection(replies[0])==projection(replies[1]),(op,projection(replies[0]),projection(replies[1]));return replies[1]
    for kind in roots:call(kind,['init'])
    assert both('show')['revision']==0 and both('history')==[];checks.append('versioned-defaults-and-empty-history')
    r=dict(schema_version=1,event_id='first',expected_revision=0,values=dict(keyboard_p95_ms=100,samples=5,loads=[dict(cards=50,render_p95_ms=1000)]),reason='Actual Go comparison')
    assert both('preview',r)['config']['revision']==1 and both('show')['revision']==0;checks.append('preview-without-publication')
    assert both('apply',r)['config']['revision']==1
    second={**r,'event_id':'second','expected_revision':1,'values':{**r['values'],'keyboard_p95_ms':2}}
    assert both('apply',second)['config']['revision']==2;assert both('apply',r)['config']['revision']==2 and len(both('history'))==2;checks.append('explicit-cas-and-replay-current-history')
    both('apply',{**r,'event_id':'stale'},False);both('apply',{**r,'reason':'Different intent'},False);assert both('show')['revision']==2;checks.append('stale-and-event-conflict-no-change')
    # Actual Go and native wire normalization, not a raw serde assumption.
    unicode_request={**r,'event_id':'wire-unicode','expected_revision':2,'reason':'Owned '+chr(0xd800)+' unicode'}
    unicode_preview=both('preview',json.dumps(unicode_request,ensure_ascii=True).encode());assert unicode_preview['config']['history'][-1]['change']['reason']=='Owned '+chr(0xfffd)+' unicode'
    originals={kind:(root/'.swarm/graph-performance.json').read_bytes() for kind,root in roots.items()}
    value=dict(keyboard_p95_ms=100,samples=5,loads=[dict(cards=50,render_p95_ms=1000)]);base=dict(schema_version=1,revision=0,values=value,history=[])
    def wire(v):return json.dumps(v,separators=(',',':')).encode()
    cases=[('valid-control',wire(base),True),('null-history',wire({**base,'history':None}),True),('missing-values',b'{"schema_version":1,"revision":0}',False),('null-values',wire({**base,'values':None}),False),('empty-loads',wire({**base,'values':{**value,'loads':[]}}),False),('duplicate-loads',wire({**base,'values':{**value,'loads':value['loads']*2}}),False),('max-exact',wire({**base,'values':{**value,'samples':9007199254740991}}),True),('unsafe',wire({**base,'values':{**value,'samples':9007199254740992}}),False),('duplicate-merge',b'{"schema_version":1,"revision":0,"values":{"keyboard_p95_ms":100,"samples":5,"loads":[{"cards":50,"render_p95_ms":1000}]},"VALUES":{"KEYBOARD_P95_MS":null,"SAMPLES":7}}',True),('earlier-wrong-type',b'{"schema_version":1,"revision":0,"values":{"samples":"wrong"},"values":{"keyboard_p95_ms":100,"samples":5,"loads":[{"cards":50,"render_p95_ms":1000}]}}',False)]
    for name,raw,success in cases:
        publish(output/(name+'.source.json'),raw)
        for kind,root in roots.items():publish(root/'.swarm/graph-performance.json',raw)
        both('show',success=success)
    checks.append('ten-raw-wire-cases-null-duplicates-casefold-recipe-exact-integer')
    historical=dict(schema_version=1,revision=1,values=value,history=[dict(revision=1,values=value,actor='',at='',change=None)])
    for name,v in [('null-change',historical),('bad-header-metadata',{**historical,'history':[{**historical['history'][0],'change':dict(schema_version=9,event_id='',expected_revision=-1,values=None,reason='')}]})]:
        raw=wire(v);publish(output/(name+'.source.json'),raw)
        for kind,root in roots.items():publish(root/'.swarm/graph-performance.json',raw)
        both('show')
    for kind,root in roots.items():publish(root/'.swarm/graph-performance.json',originals[kind])
    assert both('show')['revision']==2;checks.append('historical-metadata-not-rejudged-original-source-recovered')
    publish(output/'result.json',dict(status='PASS',checks=checks,raw_wire_cases=13,native_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),roots={k:str(v) for k,v in roots.items()},scope='bounded public differential; not global equivalence or provider acceptance'))
    print('GRAPH_PERFORMANCE_GO_PARITY_VERIFIED',len(checks),'WIRE13',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
