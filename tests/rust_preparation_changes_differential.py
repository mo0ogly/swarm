#!/usr/bin/env python3
"""Compare retained Rust review changes with the original Go pure producer.

Read the explicitly pinned Git object, extract exact types/functions, and run
only a stdlib harness in a new evidence directory. Go is a test oracle, never a
product dependency or live datastore writer. No model/network/global watchdog.
"""
import argparse,hashlib,json,os,re,subprocess
from pathlib import Path
from rust_agent_model_route import publish

def main():
    """Keep source fragments, Git ref, versions, argv and both actual outputs."""
    ap=argparse.ArgumentParser();ap.add_argument('--journey',required=True);ap.add_argument('--go-ref',required=True);ap.add_argument('--output',required=True);a=ap.parse_args();journey=Path(a.journey).resolve();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);os.chmod(out,0o700)
    ref=subprocess.check_output(['git','rev-parse',a.go_ref+'^{commit}']).decode().strip();specs={'internal/engine/plan_contract.go':[('type','PlanQuestion'),('type','PlanMission'),('type','ActionPlan')],'internal/engine/product_plan.go':[('type','ProductPlan'),('type','UserJourney'),('type','UserStory'),('func','productTaskContract'),('func','productTaskChanged')],'internal/engine/prephase_revision.go':[('type','PreparationPlanChange'),('func','preparationPlanChanges')],'internal/engine/agent_exchange.go':[('func','containsString')]};parts=[];inputs={}
    for name,items in specs.items():
        raw=subprocess.check_output(['git','show',ref+':'+name]);publish(out/Path(name).name,raw);text=raw.decode();inputs[name]=dict(git_sha256=hashlib.sha256(raw).hexdigest(),fragments={})
        for kind,symbol in items:
            prefix='type '+symbol+' struct' if kind=='type' else 'func '+symbol
            match=re.search(r'^'+re.escape(prefix)+r'[^\n]*\{.*?^}',text,re.M|re.S);assert match,(name,symbol);fragment=match.group();parts.append(fragment);entry=dict(sha256=hashlib.sha256(fragment.encode()).hexdigest())
            if Path(name).is_file():
                current=re.search(r'^'+re.escape(prefix)+r'[^\n]*\{.*?^}',Path(name).read_text(),re.M|re.S);assert current and current.group()==fragment,(name,symbol,'local Go differs from oracle fragment');entry['current_fragment_equal']=True
            inputs[name]['fragments'][symbol]=entry
    cases=[]
    for index,command in enumerate(json.loads((journey/'commands.json').read_text()),1):
        argv=command['argv']
        if len(argv)>2 and argv[1:3]==['prepare','release-review'] and command['exit']==0:
            raw=(journey/('cli-'+str(index)+'.stdout')).read_bytes();view=json.loads(raw)
            if view.get('revision'):cases.append(dict(old=view['preparation']['conversion']['spec'],next=view['revision']['spec'],expected=view['revision']['changes'],source=str(journey/('cli-'+str(index)+'.stdout')),source_sha256=hashlib.sha256(raw).hexdigest()))
    assert cases,'No executed Rust comparison retained';unique={json.dumps((c['old'],c['next']),sort_keys=True) for c in cases};publish(out/'cases.json',cases)
    harness='package main\nimport("encoding/json";"fmt";"os";"reflect")\n'+'\n'.join(parts)+'''\nfunc main(){ var cases []struct{Old ActionPlan `json:"old"`; Next ActionPlan `json:"next"`}; decoder:=json.NewDecoder(os.Stdin);decoder.DisallowUnknownFields();if e:=decoder.Decode(&cases);e!=nil{fmt.Fprintln(os.Stderr,e);os.Exit(2)}; out:=[][]PreparationPlanChange{};for _,c:=range cases{out=append(out,preparationPlanChanges(c.Old,c.Next))};if e:=json.NewEncoder(os.Stdout).Encode(out);e!=nil{fmt.Fprintln(os.Stderr,e);os.Exit(2)}}\n''';publish(out/'oracle.go',harness.encode())
    version=subprocess.run(['go','version'],capture_output=True);publish(out/'go-version.stdout',version.stdout);publish(out/'go-version.stderr',version.stderr);assert version.returncode==0
    argv=['go','run',str(out/'oracle.go')];env=dict(os.environ,GO111MODULE='off',GOTOOLCHAIN='local',GOPROXY='off');result=subprocess.run(argv,input=json.dumps([dict(old=c['old'],next=c['next']) for c in cases]).encode(),capture_output=True,env=env);publish(out/'go.stdout',result.stdout);publish(out/'go.stderr',result.stderr);assert result.returncode==0,result.stderr.decode();got=json.loads(result.stdout);assert len(got)==len(cases)
    for i,(actual,case) in enumerate(zip(got,cases)):
        assert actual==case['expected'],(i,actual,case['expected']);print('PASS preparation-changes-go-rust-'+str(i+1),flush=True)
    publish(out/'result.json',dict(status='PASS',go_ref=ref,original_inputs=inputs,harness_sha256=hashlib.sha256(harness.encode()).hexdigest(),argv=argv,cases=len(cases),unique_pairs=len(unique),scope='Only ordered change DTO from actual Rust CLI reviews versus pinned original Go producer; not full Go store/execution/authorization parity',provider_calls=0));print('PREPARATION_CHANGES_DIFFERENTIAL_PASS '+str(len(cases)))
if __name__=='__main__':main()
