#!/usr/bin/env python3
"""Verify a pinned workspace refuses an owned synthetic inode substitution.
A real healthy native control precedes the injected swap. The shim runs only
inside a disposable driver; no live Root, account, real secret or DB is touched.
No retry, cleanup of retained originals or whole-test watchdog is used.
"""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
import rust_agent_model_route as proof
import rust_biba_linux_boundary as control


def run(driver,out):
    """Retain the exact driver/shim, valid witness, injected effect and refusal.
    Compiler/IO/kernel failures propagate; a common refusal without a proven
    swap is not PASS. The private synthetic bytes must remain unchanged.
    """
    out.mkdir(parents=True,exist_ok=False)
    control.run(driver,out/'valid-control')
    source=Path(__file__).resolve().parent/'fixtures/biba_workspace_swap.c';shim=out/'workspace-swap.so'
    argv=['cc','-shared','-fPIC',str(source),'-ldl','-o',str(shim)]
    proof.publish(out/'compiler-argv.json',argv)
    compiled=subprocess.run(argv,capture_output=True)
    proof.publish(out/'compiler.stdout',compiled.stdout);proof.publish(out/'compiler.stderr',compiled.stderr)
    assert compiled.returncode==0,compiled.stderr
    root=Path(tempfile.mkdtemp(prefix='swarm-biba-swap-'));private=root/'.swarm';private.mkdir();workspace=root/'out';workspace.mkdir()
    synthetic=private/'synthetic';original=b'UNCHANGED_SYNTHETIC'
    proof.publish(synthetic,original);script=root/'provider.py'
    proof.publish(script,b"with open('alias','wb') as f:f.write(b'CORRUPTED_SYNTHETIC')\n")
    proof.publish(out/'fixture.json',dict(root=str(root),driver_sha256=proof.digest(driver.read_bytes()),shim_source_sha256=proof.digest(source.read_bytes()),shim_sha256=proof.digest(shim.read_bytes()),scope='Owned synthetic inode swap only'))
    env={**os.environ,'LD_PRELOAD':str(shim),'BIBA_FIXTURE_WORKSPACE':str(workspace),'BIBA_FIXTURE_SYNTHETIC':str(synthetic)}
    argv=[str(driver),str(root),str(workspace),str(script)]
    proof.publish(out/'driver-argv.json',argv)
    r=subprocess.run(argv,env=env,capture_output=True)
    proof.publish(out/'driver.stdout',r.stdout);proof.publish(out/'driver.stderr',r.stderr)
    assert workspace.with_name('out.retained').is_dir() and (workspace/'alias').samefile(synthetic)
    assert r.returncode!=0 and b'biba-workspace-instance-changed' in r.stderr
    assert synthetic.read_bytes()==original and not r.stdout
    proof.publish(out/'result.json',dict(status='PASS',swap_observed=True,private_original_unchanged=True,provider_not_executed=True,driver_sha256=proof.digest(driver.read_bytes()),product_qualified=False,installed=False))
    print('PASS workspace-inode-substitution-refused-before-provider',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--driver',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();run(a.driver.resolve(),a.output.resolve())
