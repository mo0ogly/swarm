"""Read-only score differential against actual extracted Go, plus saved-policy effect.
No watchdog, providers or live database. Each injected fault has a valid control.
"""
import copy,hashlib,json,os,subprocess,tempfile
from pathlib import Path

def qualify(root,output,native,reference):
    if output.exists():raise RuntimeError('Fresh score evidence directory required')
    output.mkdir(parents=True)
    fixture=Path(tempfile.mkdtemp(prefix='.swarm-score-',dir=Path.home()))
    (fixture/'proof.txt').write_bytes(b'proof')
    digest=hashlib.sha256(b'proof').hexdigest()
    good=dict(method_version='2',scope_id='task',artifacts={'proof.txt':digest},domains={'quality':1},checks=[dict(id='score',domain='quality',mandatory=False,gate='delivery',severity='minor',penalty=20,max_penalty=100)],results=[dict(id='score',status='PASS',count=0,evidence=['proof.txt'])])
    records=[]
    def run(label,binary,args,data=None):
        p=subprocess.run([str(binary),*args],input=data,capture_output=True,cwd=root)
        for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
        records.append(dict(label=label,exit=p.returncode))
        return p
    def compare(label,doc,phase='delivery',reject=False,raw=None):
        raw=raw if raw is not None else json.dumps(doc,ensure_ascii=False,separators=(',',':')).encode()
        file=output/(label+'.input.json');file.write_bytes(raw)
        g=run(label+'-go',reference,[],b'{"Root":'+json.dumps(str(fixture)).encode()+b',"Phase":'+json.dumps(phase).encode()+b',"Document":'+raw+b'}')
        n=run(label+'-native',native,['evaluate','--root',str(fixture),'--phase',phase,'--input',str(file),'--json'])
        assert g.returncode==n.returncode,(label,'exit mismatch',g.returncode,n.returncode)
        if reject:
            assert n.returncode==2 and not n.stdout,(label,'invalid document produced result')
        else:
            assert n.returncode in [0,1],(label,g.stderr,n.stderr)
            assert not n.stderr and not g.stderr,label
            assert json.loads(n.stdout)==json.loads(g.stdout),(label,'evaluation mismatch',json.loads(g.stdout),json.loads(n.stdout))
        assert file.read_bytes()==raw and (fixture/'proof.txt').read_bytes()==b'proof',label
    cases=[]
    for status,count in [('PASS',0),('FAIL',1),('PARTIEL',0),('NON TESTÉ',0),('NON APPLICABLE',0)]:
        for phase in ['entry','validation','delivery','audit']:
            d=copy.deepcopy(good);d['results'][0].update(status=status,count=count,reason='Not in scope')
            cases.append((status+'-'+phase,d,phase,False,None))
    for severity in ['none','minor','major','critical']:
        for mandatory in [False,True]:
            d=copy.deepcopy(good);d['checks'][0].update(severity=severity,mandatory=mandatory);d['results'][0].update(status='FAIL',count=1)
            cases.append((f'severity-{severity}-{mandatory}',d,'delivery',False,None))
    for penalty in [0,19.995,20,30,50,100,1e305]:
        d=copy.deepcopy(good);d['checks'][0]['penalty']=penalty;d['results'][0].update(status='FAIL',count=1)
        cases.append((f'penalty-{penalty}',d,'delivery',False,None))
    d=copy.deepcopy(good);d['domains']={'a':1,'z':2,'unused':9};d['checks'][0]['domain']='a';d['checks'].append(dict(d['checks'][0],id='other',domain='z'));d['results'].append(dict(d['results'][0],id='other',status='FAIL',count=1));cases.append(('weighted',d,'delivery',False,None))
    d=copy.deepcopy(good);d['results']=[];cases.append(('missing-result',d,'delivery',False,None))
    d=copy.deepcopy(good);d['checks'][0].pop('domain');cases.append(('unscored',d,'entry',False,None))
    d=copy.deepcopy(good);d['checks'][0]['domain']=None;cases.append(('null-domain',d,'audit',False,None))
    d=copy.deepcopy(good);d['results'][0]['evidence']=['./proof.txt'];cases.append(('evidence-alias',d,'delivery',False,None))
    # Unknown check metadata participates in config_digest with exact number lexemes.
    base=json.dumps(good,ensure_ascii=False,separators=(',',':')).encode()
    for name,fragment in [('lexemes',b'[1.0,-0,1e+09,1e999]'),('unicode',b'{"<>&\\u2028\\ud800":"\\udfff","z":"\xff"}'),('duplicates',b'{"key":1,"key":2}'),('deep',b'['*9500+b'0'+b']'*9500)]:
        raw=base.replace(b'"max_penalty":100',b'"extra":'+fragment+b',"max_penalty":100')
        cases.append((name,good,'delivery',False,raw))
    faults=[]
    for status in ['FAIL|PARTIEL','PASS|FAIL','PASS ','','UNKNOWN']:
        d=copy.deepcopy(good);d['results'][0].update(status=status,count=1);faults.append(('status-'+str(len(faults)),d,None))
    for key,value in [('domains',{}),('domains',{'quality':0}),('domains',{'quality':1e308}),('checks',[]),('checks',None),('results',None),('artifacts',{}),('artifacts',{'proof.txt':'stale'}),('artifacts',{'../outside':digest}),('scope_id',' '),('method_version','3')]:
        d=copy.deepcopy(good);d[key]=value;faults.append((key+'-'+str(len(faults)),d,None))
    for key,value in [('mandatory','true'),('domain','unknown'),('penalty',-1),('max_penalty',None),('severity','urgent'),('gate','audit')]:
        d=copy.deepcopy(good);d['checks'][0][key]=value;faults.append(('check-'+key,d,None))
    for key,value in [('count',1),('evidence',None),('evidence',[]),('evidence',[42]),('penalty',0),('id','unknown')]:
        d=copy.deepcopy(good);d['results'][0][key]=value;faults.append(('result-'+key,d,None))
    for key in ['checks','results']:
        d=copy.deepcopy(good);d[key].append(copy.deepcopy(d[key][0]));faults.append(('duplicate-'+key,d,None))
    faults.append(('excessive-depth',good,base.replace(b'"max_penalty":100',b'"extra":'+b'['*10001+b'0'+b']'*10001+b',"max_penalty":100')))
    faults.append(('trailing-document',good,base+b'{}'))
    faults.append(('non-object',good,b'null'))
    for label,d,phase,reject,raw in cases:compare(label,d,phase,reject,raw)
    for label,d,raw in faults:
        compare(label+'-control',good);compare(label,d,reject=True,raw=raw);compare(label+'-recovery',good)
    # Real public settings preview/apply followed by independent evaluator invocations.
    def command(label,args):
        p=run(label,native,[*args,'--root',str(fixture),'--json']);assert p.returncode==0,(label,p.stderr);return json.loads(p.stdout)
    command('native-init',['init']);document=command('policy-before',['engine-settings','show'])
    current=copy.deepcopy(good);current['method_version']='3';current['results'][0].update(status='FAIL',count=1)
    gate=output/'current.input.json';gate.write_text(json.dumps(current))
    first=command('current-default',['evaluate','--input',str(gate)])
    assert first['evaluation']['ship_allowed'] and first['evaluation']['quality']==80
    change=dict(schema_version=document['schema_version'],event_id='score-policy-raise',expected_revision=document['revision'],values=copy.deepcopy(document['values']),reason='Actual persisted evaluator policy')
    change['values']['quality_delivery_min_percent']='90'
    request=output/'policy-change.json';request.write_text(json.dumps(change))
    original=(fixture/'.swarm/engine-settings.json').read_bytes()
    preview=command('policy-preview',['engine-settings','preview','--input',str(request)]);assert original==(fixture/'.swarm/engine-settings.json').read_bytes()
    change['preview_token']=preview['preview_token'];request.write_text(json.dumps(change));command('policy-apply',['engine-settings','apply','--input',str(request)])
    saved=(fixture/'.swarm/engine-settings.json').read_bytes()
    after=run('current-raised',native,['evaluate','--input',str(gate),'--root',str(fixture),'--json']);assert after.returncode==1 and not after.stderr
    result=json.loads(after.stdout);assert not result['evaluation']['ship_allowed'] and result['policy']['generation']=='1'
    assert first['evaluation']['config_digest']!=result['evaluation']['config_digest'] and (fixture/'.swarm/engine-settings.json').read_bytes()==saved
    # Historical format remains a compatibility calculation, not task authorization.
    compare('historical-after-policy-change',good)
    summary=dict(status='PASS',scope='read-only calculation and public CLI; not gate acceptance',differential_cases=len(cases)+3*len(faults)+1,faults=len(faults),fixture=str(fixture),commands=records,policy_saved_and_consumed=True,source_of_reference='actual Go AST declarations',global_watchdog=None)
    (output/'result.json').write_text(json.dumps(summary,indent=2)+'\n')
    return summary
