#!/usr/bin/env python3
"""Compare the production pure guide matcher with pinned retained Go fragments.

No Go product is restored, no provider/database is used and no watchdog/retry is
added. The finite corpus is a regression set, not a proof for every URL string.
"""
import argparse,hashlib,itertools,json,os,subprocess
from pathlib import Path
from rust_agent_model_route import publish


def main():
    """Retain exact source fragments, argv, raw output and all 824 comparisons."""
    ap=argparse.ArgumentParser();ap.add_argument('--output',required=True);a=ap.parse_args()
    out=Path(a.output).resolve();out.mkdir(mode=0o700,parents=True,exist_ok=False)
    def run(label,argv,raw=None,environment=None):
        """Run once without a deadline; preserve original channels before refusal."""
        publish(out/(label+'.argv.json'),argv)
        r=subprocess.run(argv,input=raw,capture_output=True,env=environment)
        publish(out/(label+'.stdout'),r.stdout);publish(out/(label+'.stderr'),r.stderr)
        assert r.returncode==0,(label,r.returncode)
        return r.stdout
    ref=run('resolve',['git','rev-parse','ba581aec6e37d850a4838e20841f53b3e63824be^']).decode().strip()
    original=run('original',['git','show',ref+':internal/engine/project_memory.go'])
    text=original.decode();start=text.index('var memoryGuideLinks');end=text.index('\n}',text.index('func filteredMemoryGuide',start))+2;fragment=text[start:end]
    harness='package main\nimport("encoding/json";"os";"net/url";"path/filepath";"regexp";"strings")\n'+fragment+'''\nfunc main(){var values []string;if err:=json.NewDecoder(os.Stdin).Decode(&values);err!=nil{panic(err)};paths:=map[string]bool{".claude/field-guide/g.md":true,".claude/field-guide/secret note.md":true,".claude/field-guide/é.md":true};out:=[]bool{};for _,body:=range values{out=append(out,filteredMemoryGuide(body,paths)!=body)};if err:=json.NewEncoder(os.Stdout).Encode(out);err!=nil{panic(err)}}\n'''
    publish(out/'oracle.go',harness.encode())
    source=Path('rust/crates/swarm-engine/src/project_memory_guide.rs').resolve();publish(out/'production.rs',source.read_bytes())
    rust='#[path="'+str(source)+'"]\nmod guide;\n'+'''fn main(){use std::io::{self,BufRead};let paths=[".claude/field-guide/g.md",".claude/field-guide/secret note.md",".claude/field-guide/é.md"].map(str::to_string).into_iter().collect();for line in io::stdin().lock().lines(){let line=line.unwrap();let raw=(0..line.len()).step_by(2).map(|i|u8::from_str_radix(&line[i..i+2],16).unwrap()).collect::<Vec<_>>();let body=String::from_utf8(raw).unwrap();println!("{}",guide::guide_references_managed(&body,&paths));}}\n'''
    publish(out/'probe.rs',rust.encode())
    paths=['g.md','secret%20note.md','%C3%A9.md','%67.md','g.md?x','g.md#x','g.md#%zz','g.md?%zz','g.md\r','g%2emd','%2e%2e/field-guide/g.md','g.md%00','%FF','']
    refs=[pre+name for pre,name in itertools.product(['','./','x/../','.claude/field-guide/','../field-guide/','//','///','https://','file:','1a:','//@/'],paths)]
    cases=['Plain text','text .claude/field-guide/g.md']+[f'{prefix}{ref}{suffix}' for prefix,suffix in [('[x](',')'),('[x](<','>)'),('[x]( ',' )'),('[[x](',')'),('[x](',') trailing [b](other.md)')] for ref in refs]
    cases+=['[x]('+host+name+')' for host,name in itertools.product(['//%zz@/','//%20@/','//é@/','//a@@/','//[::1]/','//:0/','//%ff@/','//a%2fb@/','a:b/','?x#','///','////','//?#'],['g.md','%67.md','secret%20note.md','.claude/field-guide/g.md'])]
    publish(out/'cases.json',cases)
    expected=json.loads(run('go',['go','run',str(out/'oracle.go')],json.dumps(cases).encode(),dict(os.environ,GO111MODULE='off',GOTOOLCHAIN='local',GOPROXY='off')))
    run('compile',['rustc','--edition=2021',str(out/'probe.rs'),'-o',str(out/'probe')])
    got=[v=='true' for v in run('rust',[str(out/'probe')],'\n'.join(c.encode().hex() for c in cases).encode()).decode().splitlines()]
    assert len(got)==len(expected)==len(cases)==824
    differences=[dict(index=i,body=c,go=x,rust=y) for i,(c,x,y) in enumerate(zip(cases,expected,got)) if x!=y]
    publish(out/'differences.json',differences);assert not differences,differences
    publish(out/'result.json',dict(status='PASS',cases=len(cases),go_ref=ref,original_sha256=hashlib.sha256(original).hexdigest(),fragment_sha256=hashlib.sha256(fragment.encode()).hexdigest(),production_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),global_watchdog=None,scope='Pure finite Unix URL/reference corpus only, not database or full source admission parity'))
    print('PROJECT_MEMORY_GUIDE_DIFFERENTIAL_PASS '+str(len(cases)),flush=True)

if __name__=='__main__':main()
