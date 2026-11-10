from pathlib import Path
import subprocess,json,os,time
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260/quality1278');b.mkdir(mode=0o700)
env={**os.environ,'CARGO_TARGET_DIR':str(Path('.cache/rust-native-version846-target').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
commands=[('status-engine',['cargo','test','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-engine','--lib','mission_status','--','--nocapture']),('cli',['cargo','test','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-cli','--lib','--','--nocapture']),('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-cli','-p','swarm-web','--all-targets','--','-D','warnings']),('rustdoc',['cargo','doc','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-engine','-p','swarm-cli','-p','swarm-web','--no-deps'])]
commands=[next(c for c in commands if c[0]==name) for name in ['clippy','status-engine','cli','rustdoc']]
for name,cmd in commands:
 start=time.time()
 with (b/(name+'.stdout')).open('x') as out,(b/(name+'.stderr')).open('x') as err:p=subprocess.run(cmd,env=env,stdout=out,stderr=err)
 (b/(name+'.result.json')).write_text(json.dumps(dict(argv=cmd,exit=p.returncode,seconds=time.time()-start,global_watchdog=None),indent=2)+'\n');print(name,p.returncode,flush=True)
 if p.returncode:print((b/(name+'.stdout')).read_text()[-1800:]+(b/(name+'.stderr')).read_text()[-1800:],flush=True);raise SystemExit(p.returncode)
