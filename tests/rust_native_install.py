#!/usr/bin/env python3
"""Exercise the actual native installer on an owned, retained fixture.

No Go, Docker daemon, real mission or provider. Preserve exact subprocess channels,
installed version/hash, public init/work persistence and bundled method reads.
Missing output parents are created; an existing output is refused without overwrite.
A failed prerequisite stops the recipe; no watchdog or implicit retry.
"""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
    """Install through the public shell into a fresh output and verify actual CLI."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);a=parser.parse_args();out=a.output.resolve();out.mkdir(mode=0o700,parents=True,exist_ok=False)
    project=out/'project with spaces';project.mkdir();(project/'keep.txt').write_text('preserved');bin_dir=out/'native bin';calls=[];checks=[]
    def call(args,value=None):
        """Retain original argv/channels, stop on failure, never print private data."""
        index=len(calls)+1;args=[*args,*(['--input','-'] if value is not None else [])];r=subprocess.run(args,cwd=ROOT,input=None if value is None else json.dumps(value).encode(),capture_output=True)
        for suffix,data in [('stdout',r.stdout),('stderr',r.stderr)]: (out/(str(index)+'.'+suffix)).write_bytes(data)
        calls.append(dict(argv=args,exit=r.returncode));(out/'commands.json').write_text(json.dumps(calls,indent=2));assert r.returncode==0,(index,r.returncode);return r.stdout
    def check(name,value):
        """Record named meaningful assertions; any failed assertion propagates."""
        assert value,name;assert name not in checks;checks.append(name);print('PASS native-install-'+name,flush=True)
    call(['./install.sh','--mode','native','--bin-dir',str(bin_dir),'--project',str(project)])
    binary=bin_dir/'swarm'
    def cli(args,value=None):
        """Decode actual installed CLI output using the owned Root only."""
        return json.loads(call([str(binary),*args,'--root',str(project),'--json'],value))
    version=cli(['version']);check('installed-version',version['binary']['provenance']=='injected' and version['binary']['version']=='devel' and len(version['binary']['commit'])==40)
    check('fresh-root-empty',cli(['work','list'])==[])
    methods=cli(['prepare','methods']);check('bundled-methods',len(methods)==5 and all(m['available'] and len(m['sha256'])==64 for m in methods));check('no-local-method-files',not(project/'.claude').exists())
    created=cli(['work','create'],dict(schema_version=1,event_id='install-fixture',expected_revision=0,title='Installation Rust',objective='Persist through fresh CLI processes',scope='Isolated fixture',criteria=['Preserve the recorded work']))['work'];check('public-work-persisted',cli(['work','show',created['id']])['work']==created)
    check('project-preserved',(project/'keep.txt').read_text()=='preserved');check('binary-executable',os.access(binary,os.X_OK));check('private-database',((project/'.swarm/state.db').stat().st_mode&0o777)==0o600)
    result=dict(status='PASS',checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),version=version['binary'],scope='Real native install and fresh CLI only; no Docker, web runtime or full migration acceptance',global_watchdog=None)
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print('RUST_NATIVE_INSTALL_PASS '+str(len(checks))+' assertions',flush=True)
if __name__=='__main__':main()
