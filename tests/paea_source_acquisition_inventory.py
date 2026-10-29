#!/usr/bin/env python3
"""Run the PAEA source-acquisition contract against prebound actual Cargo inputs.

Candidates are frozen before no-run compilation; actual workspace depfiles must
be a byte-identical subset. Later unrelated file births cannot silently become
compiler inputs. The recipe runner then hashes explicit frozen paths, preserving
its normal changed/removed-input guard. No fixture features, source checkout copy,
policy bypass, command retry, installation or global watchdog is introduced.
"""
import argparse,fcntl,importlib.util,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def module(name,path):
    """Load the existing retained verification producer; errors propagate."""
    spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value

PROFILE=module('paea_profile',ROOT/'tools/verification/verify_native_profile.py')
RECIPE=module('paea_recipe',ROOT/'tools/verification/recipe.py')

def workspace_inputs(events,out,index):
    """Read real workspace library/test depfiles from successful Cargo artifacts.

    Reuse the native profiler's observed Linux Make parser, with Cargo's Rust
    workspace cwd. Missing/unsupported workspace outputs refuse, never an empty
    inventory. Registry crates remain bound by lockfile/artifact metadata scope.
    Raw depfile bytes are retained for diagnostics and line continuations are
    refused explicitly: the shared parser reads only its first line, so a
    continuation would silently truncate the effective input set instead of
    being admitted. UTF-8 invalid bytes are kept via surrogateescape so the
    retained .raw stays byte-faithful while the check still runs.
    """
    actual={};executables=[];depfiles=[]
    for event in events:
        if event.get('reason')!='compiler-artifact':continue
        PROFILE.require(not set(event['features'])&{'test-fixtures','test-util'},'acquisition-fixture-feature')
        manifest=Path(event['manifest_path']).resolve()
        if not manifest.is_relative_to(ROOT/'rust/crates'):continue
        executable=event.get('executable')
        if executable:
            file=Path(executable);depfile=file.with_name(file.name+'.d');executables.append(str(file))
        else:
            libraries=[Path(p) for p in event['filenames'] if p.endswith('.rlib')]
            PROFILE.require(len(libraries)==1,'acquisition-workspace-library-output')
            file=libraries[0];PROFILE.require(file.name.startswith('lib'),'acquisition-library-name');depfile=file.with_name(file.name[3:-5]+'.d')
        PROFILE.require(depfile.is_file(),'acquisition-depfile-absent')
        raw=depfile.read_bytes()
        PROFILE.publish(out/f'compile-{index}-depfile-{len(depfiles):03d}.raw',raw)
        PROFILE.require(all(not line.endswith('\\') for line in raw.decode('utf-8','surrogateescape').splitlines()),'acquisition-depfile-continuation-unsupported')
        previous=Path.cwd()
        try:
            os.chdir(ROOT/'rust');inputs=PROFILE.dependencies(ROOT,depfile)
        finally:os.chdir(previous)
        for path,digest in inputs.items():
            PROFILE.require(path not in actual or actual[path]==digest,'acquisition-input-changed-between-artifacts');actual[path]=digest
        depfiles.append(str(depfile))
    PROFILE.require(bool(actual) and bool(executables),'acquisition-no-compiled-target')
    return actual,executables,depfiles

def run(recipe_path,out,unit_only):
    """Hold the cooperative build lock through preflight and named assertions.

    Fresh evidence retains original recipes, prebound bytes, no-run argv/output,
    actual depfiles and executable hashes. No-run is compilation only; named case
    outcomes belong to the unchanged recipe runner. Any failed prerequisite stops
    before tests, and all prior evidence remains intact.
    """
    out.mkdir(parents=True,exist_ok=False)
    with (ROOT/'deploy/.local-build.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        recipe=json.loads(recipe_path.read_text());PROFILE.publish(out/'original-recipe.json',recipe_path.read_bytes())
        before=PROFILE.candidates(ROOT);before.update(RECIPE.snapshot(ROOT,recipe['inputs'],out,recipe_path))
        for method in [Path(__file__).resolve(),Path(PROFILE.__file__).resolve(),Path(RECIPE.__file__).resolve()]:before[str(method.relative_to(ROOT))]=PROFILE.digest(method)
        PROFILE.publish(out/'candidates.before.json',json.dumps(before,indent=2)+'\n')
        PROFILE.archive_effective_sources(ROOT,out,before)
        env={**os.environ,**{k:v.replace('{root}',str(ROOT)) for k,v in recipe.get('environment',{}).items()}}
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
        PROFILE.require(not any(env.get(k) for k in names),'acquisition-ambient-compiler-override')
        # LIBSQLITE3_FLAGS reaches native C builds that Rust depfiles cannot
        # observe; refuse it unless the recipe declares it explicitly so the
        # ambient value can never leak into a "compiler-bound" PASS unbound.
        PROFILE.require(os.environ.get('LIBSQLITE3_FLAGS') is None or 'LIBSQLITE3_FLAGS' in recipe.get('environment',{}),'acquisition-ambient-native-flags-undeclared')
        commands=[['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-engine','--lib','--no-run','--message-format=json']]
        if not unit_only:commands.append(['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-engine','--test','paea_git_repository','--no-run','--message-format=json'])
        if unit_only:
            # Only the --lib unit is precompiled and hashed here; any --test or
            # doc step in the frozen recipe would recompile or run without a
            # depfile bound, so --unit-only must refuse instead of over-claiming.
            unbound=[step['id'] for step in recipe['steps'] if '--test' in step['command'] or 'doc' in step['command']]
            PROFILE.require(not unbound,'acquisition-unit-only-unbound-named-targets:'+','.join(unbound))
        PROFILE.publish(out/'invocation.json',json.dumps({'commands':commands,'cwd':str(ROOT),'unit_only':unit_only,'global_watchdog':None,'environment':recipe.get('environment',{}),'ambient_libsqlite3_flags':os.environ.get('LIBSQLITE3_FLAGS') is not None},indent=2)+'\n')
        actual={};bins={}
        for index,command in enumerate(commands,1):
            stdout=out/f'compile-{index}.stdout';stderr=out/f'compile-{index}.stderr'
            with stdout.open('xb') as output,stderr.open('xb') as error:result=subprocess.run(command,cwd=ROOT,env=env,stdin=subprocess.DEVNULL,stdout=output,stderr=error)
            PROFILE.require(result.returncode==0,'acquisition-no-run-compilation-failed')
            events=[json.loads(line) for line in stdout.read_text().splitlines() if line.startswith('{')]
            PROFILE.require(any(e.get('reason')=='build-finished' and e.get('success') is True for e in events),'acquisition-build-finished-missing')
            inputs,executables,depfiles=workspace_inputs(events,out,index);PROFILE.effective_unchanged(ROOT,before,inputs);actual.update(inputs)
            PROFILE.publish(out/f'compile-{index}-depfiles.json',json.dumps(depfiles,indent=2)+'\n')
            for file in executables:bins[file]=PROFILE.digest(Path(file))
        PROFILE.effective_unchanged(ROOT,before,actual)
        frozen={**recipe,'inputs':sorted(before)};PROFILE.publish(out/'frozen-recipe.json',json.dumps(frozen,indent=2)+'\n');PROFILE.publish(out/'actual-compiled-inputs.json',json.dumps(actual,indent=2)+'\n');PROFILE.publish(out/'executables.before.json',json.dumps(bins,indent=2)+'\n')
        with (out/'recipe.stdout').open('xb') as output,(out/'recipe.stderr').open('xb') as error:
            result=subprocess.run([sys.executable,str(ROOT/'tools/verification/recipe.py'),'run','--recipe',str(out/'frozen-recipe.json'),'--output',str(out/'checks')],cwd=ROOT,env=env,stdin=subprocess.DEVNULL,stdout=output,stderr=error)
        PROFILE.require(result.returncode==0,'acquisition-named-recipe-refused')
        PROFILE.effective_unchanged(ROOT,before,actual)
        PROFILE.require(all(PROFILE.digest(Path(file))==digest for file,digest in bins.items()),'acquisition-tested-executable-changed')
        # Publish the post-recipe ELF hashes explicitly: until now a stable
        # second digest was only implied by the absence of the refusal above.
        PROFILE.publish(out/'executables.after.json',json.dumps(bins,indent=2)+'\n')
        PROFILE.publish(out/'late-candidates.json',json.dumps(sorted(set(PROFILE.candidates(ROOT))-set(before)),indent=2)+'\n')
        scope='Compiler-bound named PAEA checks only; not service or independent acceptance'
        if any('doc' in step['command'] for step in recipe['steps']):scope+='; rustdoc step bound by declared-source stability only, not by a compile-bound depfile'
        PROFILE.publish(out/'result.json',json.dumps({'status':'PASS','scope':scope,'compiled_inputs':len(actual),'frozen_declared_inputs':len(before),'unit_only':unit_only,'global_watchdog':None,'executables':bins},indent=2)+'\n')

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--recipe',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--unit-only',action='store_true');args=parser.parse_args()
    try:run(args.recipe.resolve(),args.output.resolve(),args.unit_only)
    except Exception as error:
        # Retain a machine-readable refusal next to any prior evidence; PASS
        # remains established only by a published result.json in the same run.
        output=args.output.resolve()
        try:
            if output.is_dir():PROFILE.publish(output/'failure.json',json.dumps({'status':'REFUSED','type':type(error).__name__,'reason':str(error),'result_json_published':(output/'result.json').exists(),'scope':'Refusal retained; prior evidence intact; PASS exists only with result.json'},indent=2)+'\n')
        except Exception:pass
        print('PAEA_COMPILER_BOUND_REFUSED '+type(error).__name__+': '+str(error),file=sys.stderr);sys.exit(2)
