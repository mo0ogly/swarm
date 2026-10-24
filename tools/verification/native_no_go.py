#!/usr/bin/env python3
"""Run a native recipe with owned PATH traps for go/gofmt and retained evidence.

This observes PATH-dispatched tools and rejects declared Go inputs/commands. It
is not a hermetic process graph or sandbox against absolute nested tool paths.
No provider, deployment, retry or global timeout authority is created.
"""
import argparse, hashlib, json, os, subprocess, sys
from pathlib import Path
from recipe import REPOSITORY, load_recipe, publish


def run(root, recipe_path, output):
    """Execute once in fresh evidence; even ignored Go refusal blocks this claim.

    Only the owned child's PATH changes. Component results/stdout/stderr remain
    original; wrapper status additionally requires no trap marker. Existing
    outputs refuse before overwrite. File/process failures retain normal causes;
    absent/incomplete component evidence never becomes PASS.
    """
    root=Path(root).resolve();recipe_path=Path(recipe_path).resolve();output=Path(output).resolve()
    value=load_recipe(recipe_path.read_bytes())
    if any(x.endswith('.go') or Path(x).name in ('go.mod','go.sum') for x in value['inputs']):
        raise ValueError('native-recipe-declares-go-input')
    if any(Path(s['command'][0]).name in ('go','gofmt','go.exe','gofmt.exe') for s in value['steps']):
        raise ValueError('native-recipe-declares-go-command')
    if output.exists() or output in (root,root/'.git',root/'.swarm') or output.is_relative_to(root/'.git') or output.is_relative_to(root/'.swarm'):
        raise ValueError('native-proof-must-be-new-outside-live-state')
    output.mkdir(mode=0o700);trap=output/'tool-trap';trap.mkdir(mode=0o700);marker=output/'go-invocation.json'
    script=('#!'+sys.executable+'\nimport os,json,sys\nfrom pathlib import Path\n'
        +'p=Path('+repr(str(marker))+')\n'
        +'with p.open("xb") as f: f.write(json.dumps(dict(tool=Path(sys.argv[0]).name,pid=os.getpid())).encode());f.flush();os.fsync(f.fileno())\n'
        +'raise SystemExit(97)\n')
    for name in ('go','gofmt'):
        target=trap/name;publish(target,script.encode());target.chmod(0o700)
    publish(output/'wrapper.py',Path(__file__).read_bytes())
    publish(output/'invocation.json',dict(recipe=str(recipe_path),recipe_sha256=hashlib.sha256(recipe_path.read_bytes()).hexdigest(),
        root=str(root),path_traps=['go','gofmt'],inherited_path_recorded=False,global_watchdog=None))
    environment=dict(os.environ);environment['PATH']=str(trap)+os.pathsep+environment.get('PATH',os.defpath)
    command=[sys.executable,str(Path(__file__).with_name('recipe.py')),'run','--root',str(root),
        '--recipe',str(recipe_path),'--output',str(output/'component')]
    child=subprocess.run(command,env=environment,stdin=subprocess.DEVNULL)
    component=output/'component/result.json';result=json.loads(component.read_bytes()) if component.exists() else None
    observed=marker.exists();passed=child.returncode==0 and result is not None and result['status']=='PASS' and not observed
    final=dict(status='PASS' if passed else 'FAIL',component_exit=child.returncode,component_status=result['status'] if result else 'MISSING',
        go_path_invoked=observed,scope='Declared inputs/commands and observed PATH tools; not a hermetic process graph',global_watchdog=None)
    publish(output/'result.json',final)
    return final


def main():
    """Expose explicit source/output locations; refusals stay nonzero and visible."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--recipe',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path);p.add_argument('--root',type=Path,default=REPOSITORY);a=p.parse_args()
    try:
        result=run(a.root,a.recipe,a.output);print(json.dumps(result));return 0 if result['status']=='PASS' else 1
    except Exception as error:
        print(json.dumps(dict(status='REFUSED',cause=str(error),type=type(error).__name__)));return 2


if __name__=='__main__':raise SystemExit(main())
