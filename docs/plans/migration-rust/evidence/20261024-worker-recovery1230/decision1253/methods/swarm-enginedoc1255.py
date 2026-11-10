from pathlib import Path
import subprocess,os,json,time
b=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230/enginedoc1255');b.mkdir(mode=0o700)
cmd=['cargo','doc','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-engine','--no-deps'];start=time.time()
env={**os.environ,'CARGO_TARGET_DIR':str(Path('.cache/rust-native-version846-target').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
with (b/'stdout').open('x') as out,(b/'stderr').open('x') as err:p=subprocess.run(cmd,env=env,stdout=out,stderr=err)
(b/'result.json').write_text(json.dumps(dict(argv=cmd,exit=p.returncode,seconds=time.time()-start,global_watchdog=None),indent=2)+'\n');print('engine-rustdoc',p.returncode,flush=True)
raise SystemExit(p.returncode)
