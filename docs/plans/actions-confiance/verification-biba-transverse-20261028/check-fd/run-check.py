"""Check the scoped production-default engine; detect changed declared sources.
This is a compilation diagnostic, not qualification, installation or acceptance.
"""
from pathlib import Path
import json,sys,subprocess,os,hashlib
root=Path.cwd();out=Path(__file__).resolve().parent
sys.path.insert(0,str(root/'tools/rust-migration'))
from qualification_inputs import rust_assets

def library_files():
    sources=set((root/"rust").glob("crates/*/src/**/*.rs"))
    manifests=set((root/"rust").glob("**/Cargo.toml"))
    manifests={p for p in manifests if "target" not in p.parts}
    return sources | manifests | {root/"rust/Cargo.lock",root/".cargo/config.toml"} | rust_assets(root,sources)

def snapshot():
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(library_files())}
before=snapshot();(out/'check-inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
argv=['cargo','check','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-engine','--lib']
env={**os.environ,'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_INCREMENTAL':'0'}
with (out/'check-corrected.out').open('xb') as log:
    process=subprocess.Popen(argv,stdout=log,stderr=subprocess.STDOUT,env=env)
    print('cargo check pid',process.pid,flush=True);code=process.wait()
after=snapshot();(out/'check-inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');changed=sorted(k for k in before.keys()|after.keys() if before.get(k)!=after.get(k))
result=dict(exit=code,sources_stable=not changed,changed=changed,argv=argv,profile=dict(dev_debug='0',incremental='0',test_fixtures_requested=False, effective_features_not_qualified=True),qualification=False)
(out/'check-result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)
raise SystemExit(code or bool(changed))
