#!/usr/bin/env python3
"""Build the standalone fixture from Cargo's actual artifact, then verify it.
No guessed hashed rlib, feature promotion, product install, retry or watchdog.
The native boundary test is distinct from the production engine qualification.
"""
import argparse,json,os,subprocess,sys
from pathlib import Path
import rust_agent_model_route as proof


def run(target,out):
    """Bind compiler argv, artifact and channels; stop at the first failed stage.
    Only an explicit project-local target and fresh proof directory are used.
    IO, compiler and assertion failures retain originals and never emit PASS.
    """
    root=Path(__file__).resolve().parents[1]
    assert target.is_relative_to(root)
    out.mkdir(parents=True,exist_ok=False)
    def command(name,argv,env=None):
        """Keep complete invocation/bytes; process success is only a prerequisite."""
        proof.publish(out/(name+'-argv.json'),argv)
        r=subprocess.run(argv,cwd=root,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        proof.publish(out/(name+'.stdout'),r.stdout);proof.publish(out/(name+'.stderr'),r.stderr)
        assert r.returncode==0,(name,r.returncode)
        return r.stdout
    cargo=command('cargo',['cargo','build','--manifest-path',str(root/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-biba-linux','--target-dir',str(target),'--message-format=json'],{**os.environ,'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_INCREMENTAL':'0'})
    artifacts=[json.loads(line) for line in cargo.splitlines() if line.startswith(b'{')]
    selected=[v for v in artifacts if v.get('reason')=='compiler-artifact' and v['target']['name']=='swarm_biba_linux' and v['target']['kind']==['lib']]
    assert len(selected)==1 and not selected[0]['features']
    libraries=[Path(n) for n in selected[0]['filenames'] if n.endswith('.rlib')];assert len(libraries)==1
    library=libraries[0];driver=out/'boundary-driver'
    proof.publish(out/'artifact.json',dict(artifact=selected[0],sha256=proof.digest(library.read_bytes())))
    command('rustc',['rustc','--edition=2021',str(root/'tests/rust_biba_boundary_driver.rs'),'-L','dependency='+str(target/'debug/deps'),'--extern','swarm_biba_linux='+str(library),'-o',str(driver)])
    command('native',[sys.executable,str(root/'tests/rust_biba_linux_boundary.py'),'--driver',str(driver),'--output',str(out/'native')])
    assert json.loads((out/'native/result.json').read_text())['status']=='PASS'
    command('workspace-swap',[sys.executable,str(root/'tests/rust_biba_workspace_swap.py'),'--driver',str(driver),'--output',str(out/'workspace-swap')])
    swap=json.loads((out/'workspace-swap/result.json').read_text())
    assert swap['status']=='PASS' and swap['private_original_unchanged'] and swap['swap_observed'] and swap['provider_not_executed']
    print('NATIVE_BIBA_BOUNDARY_PASS',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--target-dir',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();run(a.target_dir.resolve(),a.output.resolve())
