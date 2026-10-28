#!/usr/bin/env python3
"""Public imported history vs Go JSON on frozen actual DTO declarations.

Go is a verification oracle only: no Store/provider/mission runs. An owned Root
contains private inert manifests; CLI history must never adopt their PIDs or
write SQL. No live database, implicit retry, model call or global watchdog.
"""
from pathlib import Path
from contextlib import closing
import argparse,base64,copy,hashlib,json,os,sqlite3,subprocess,traceback
from rust_agent_model_route import Fixture,publish,digest

CONTRACT=Path(__file__).resolve().parents[1]/'contracts/archive-history-wire.v1.json'
def oracle(layout):
    """Emit Go's standard typed decoder from the frozen inventoried public fields.

    No private/non-JSON fields/custom methods are invented. This proves decoder
    behavior, not a live Go Store, archive import validation or authenticity.
    Closed inputs are developer-owned contract data, not an untrusted code source.
    """
    assert layout['custom_methods']=={} and layout['external']==['json.RawMessage']
    lines=['package main\nimport("bufio";"encoding/json";"encoding/base64";"os";"io")\n']
    for name,t in sorted(layout['types'].items()):
        assert not t.get('unsupported') and not t.get('underlying')
        lines.append('type '+name+' struct {\n')
        for f in t['fields']:lines.append(f['go_name']+' '+f['go_type']+' `json:"'+f['json_tag']+'"`\n')
        lines.append('}\n')
    lines.append('''func main(){r:=bufio.NewReader(os.Stdin);e:=json.NewEncoder(os.Stdout);for{line,err:=r.ReadBytes('\u005cn');if len(line)>0{var packet struct{Raw string `json:"raw"`};if json.Unmarshal(line,&packet)!=nil{panic("oracle packet")};raw,x:=base64.StdEncoding.DecodeString(packet.Raw);if x!=nil{panic(x)};var b Bundle;if json.Unmarshal(raw,&b)!=nil{if e.Encode(map[string]any{"ok":false})!=nil{panic("output")}}else{if e.Encode(map[string]any{"ok":true,"value":map[string]any{"historical_only":true,"history":b.Cockpit}})!=nil{panic("output")}}};if err==io.EOF{return};if err!=nil{panic(err)}}}\n''')
    return ''.join(lines).encode()
def cases():
    """Named compatibility corpus and known-type refusals, not a universal proof."""
    values=[('null',b'null'),('empty',b'{}'),('absent',b'{"work":{"id":"old"}}'),('null-cockpit',b'{"cockpit_history":null}'),('zero-cockpit',b'{"cockpit_history":{}}'),('empty-collections',b'{"cockpit_history":{"agents":[],"logs":[],"controls":[],"decisions":[],"visits":[]}}')]
    seed={'schema_version':-9,'work':{'id':'historical-work','revision':-3},'cockpit_history':{'limits':'declared archive limit','agents':[{'id':'archived-running','mode':'dialogue','status':'running','supervisor_pid':os.getpid(),'child_pid':os.getpid(),'host':'untrusted archive','native_execution':{'forged':'not native'},'native_provider_facts':{'forged':'ignore'},'native_tool_guard_policy':{'forged':'ignore'},'native_execution_result':{'forged':'ignore'}}],'logs':[{'seq':9,'agent_id':'archived-running','at':'past','kind':'output','message':'<script> historical & text'}],'controls':[{'kind':'resume','message':'do not replay'},None,{}],'decisions':[{'id':'d','task_id':'old-task','kind':'accept','author':'supplied','summary':'never current authority'}],'visits':[{'revision':9223372036854775807,'at':'past','operator':'supplied'}]}}
    values.append(('inert-running-and-decisions',json.dumps(seed).encode()))
    for field in ['agents','logs','controls','decisions','visits']:
        for v,label in [(None,'null'),([],'empty'),([None],'null-value')]:
            d=copy.deepcopy(seed);d['cockpit_history'][field]=v;values.append((field+'-'+label,json.dumps(d).encode()))
    for field,wrong in [('schema_version','invalid'),('work',[]),('events',{}),('files',{'a':9}),('missing',[3]),('assistant_history',{}),('automation_state',[{'rows':[['bad']]}]),('cockpit_history',[] )]:
        d=copy.deepcopy(seed);d[field]=wrong;values.append(('invalid-bundle-'+field,json.dumps(d).encode()))
    for field,wrong in [('agents',{}),('logs',[{'seq':'wrong'}]),('controls',[{'kind':3}]),('decisions',[{'summary':False}]),('visits',[{'revision':9223372036854775808}]),('limits',7)]:
        d=copy.deepcopy(seed);d['cockpit_history'][field]=wrong;values.append(('invalid-cockpit-'+field,json.dumps(d).encode()))
    values.extend([('duplicate-merge',b'{"cockpit_history":{"limits":"first","agents":[]},"COCKPIT_HISTORY":{"limits":null,"logs":[]}}'),('duplicate-pointer-null',b'{"cockpit_history":{"limits":"first"},"cockpit_history":null}'),('unknown-forward',b'{"future":{"x":[1,2]},"cockpit_history":{"future":9}}'),('invalid-utf8-go-replacement',b'{"cockpit_history":{"limits":"\xff\xfe"}}'),('invalid-unused-work-type',b'{"work":{"revision":"bad"},"cockpit_history":{}}'),('invalid-unused-assistant-type',b'{"assistant_history":[{"context":{"revision":"bad"}}],"cockpit_history":{}}'),('invalid-automation-type',b'{"automation_state":[{"name":"x","rows":[[{"kind":9}]]}]}'),('malformed',b'{"cockpit_history":'),('trailing',b'{} {}')])
    return values

def run(binary,output,unported=False):
    """Own the fixture and oracle, require real refusals/readback without writes."""
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'archive-history');checks=[]
    imports=f.root/'.swarm/imports';imports.mkdir(mode=0o700,exist_ok=True);directory=imports/f.work;directory.mkdir(mode=0o700);manifest=directory/'manifest.json';publish(manifest,b'{"cockpit_history":{"agents":[],"logs":[],"controls":[],"limits":"inert"}}')
    def sql():
        """Read-only owned-fixture dump; close the connection on success/error."""
        with closing(sqlite3.connect((f.root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True)) as c:return '\n'.join(c.iterdump())
    before=sql();settings=(f.root/'.swarm/engine-settings.json').read_bytes()
    def check(name,value):
        assert value,name
        checks.append(name);publish(output/'checks.json',checks);print('PASS '+name,flush=True)
    check('existing-work-valid-control',f.ok(['work','show',f.work])['work']['id']==f.work)
    code,value=f.call(['agent','history',f.work])
    if unported:
        check('history-explicitly-unported',code!=0);check('unported-sql-unchanged',sql()==before);publish(output/'result.json',dict(status='GAP_OBSERVED',checks=checks));print('AGENT_ARCHIVE_HISTORY_GAP 3',flush=True);return
    check('baseline-historical-wire',code==0 and value=={'historical_only':True,'history':{'agents':[],'logs':[],'controls':[],'limits':'inert'}})
    # Legacy Go emits JSON even without the global --json presentation flag.
    argv=[str(f.binary),'agent','history',f.work,'--root',str(f.root)]
    plain=subprocess.run(argv,env=f.environment,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    publish(output/'without-json.stdout',plain.stdout);publish(output/'without-json.stderr',plain.stderr);publish(output/'without-json-command.json',dict(argv=argv,exit=plain.returncode))
    check('without-json-still-historical-json',plain.returncode==0 and json.loads(plain.stdout)==value and sql()==before)
    layout=json.loads(CONTRACT.read_bytes())['layout'];source=output/'oracle.go';publish(source,oracle(layout));exe=output/'go-json-oracle';env={**os.environ,'GOTOOLCHAIN':'local','GOPROXY':'off'}
    done=subprocess.run(['go','build','-o',str(exe),str(source)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);publish(output/'go-build.stdout',done.stdout);publish(output/'go-build.stderr',done.stderr);assert done.returncode==0,done.stderr.decode()
    corpus=cases();packets=b''.join(json.dumps({'raw':base64.b64encode(raw).decode()}).encode()+b'\n' for _,raw in corpus);done=subprocess.run([str(exe)],input=packets,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);publish(output/'go-input.jsonl',packets);publish(output/'go-output.jsonl',done.stdout);publish(output/'go-stderr',done.stderr);assert done.returncode==0 and not done.stderr
    answers=[json.loads(line) for line in done.stdout.splitlines()];assert len(answers)==len(corpus)
    for (name,raw),expected in zip(corpus,answers):
        publish(manifest,raw);code,value=f.call(['agent','history',f.work]);check('differential-'+name,code==0 and value==expected['value'] if expected['ok'] else code!=0)
        check('readonly-'+name,sql()==before and manifest.read_bytes()==raw and (f.root/'.swarm/engine-settings.json').read_bytes()==settings)
    # Missing, corrupt and replaced private paths must not become an empty list.
    valid=b'{"cockpit_history":{"agents":[],"limits":"recovered"}}';publish(manifest,valid);assert f.call(['agent','history',f.work])[0]==0
    held=directory/'held.json';manifest.rename(held);code,_=f.call(['agent','history',f.work]);check('missing-refuses',code!=0 and sql()==before);held.rename(manifest);check('missing-recovers',f.call(['agent','history',f.work])[0]==0)
    manifest.rename(held);manifest.symlink_to(held);code,_=f.call(['agent','history',f.work]);check('symlink-refuses',code!=0 and sql()==before);manifest.unlink();held.rename(manifest);check('symlink-recovers',f.call(['agent','history',f.work])[0]==0)
    for args in [['agent','history','../escape'],['agent','history','missing'],['agent','history',f.work,'extra']]:check('bad-argument-'+str(len(checks)),f.call(args)[0]!=0 and sql()==before)
    # Projected history is still available if no current Work matches its id.
    other=directory.parent/'only-archive';other.mkdir(mode=0o700);publish(other/'manifest.json',valid);code,v=f.call(['agent','history','only-archive']);check('archive-without-current-work',code==0 and v['historical_only'] is True)
    check('whole-fixture-preserved',sql()==before and (f.root/'.swarm/engine-settings.json').read_bytes()==settings)
    publish(output/'result.json',dict(status='PASS',checks=checks,cases=len(corpus),binary_sha256=digest(binary.read_bytes()),oracle_sha256=digest(exe.read_bytes()),contract_sha256=digest(CONTRACT.read_bytes()),provider_started=False,live_database=False));print('AGENT_ARCHIVE_HISTORY_VERIFIED '+str(len(checks)),flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.is_dir():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
