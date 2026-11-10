from pathlib import Path
import subprocess,json,os,time,sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230/tests1251');b.mkdir(mode=0o700)
env={**os.environ,'CARGO_TARGET_DIR':str(Path('.cache/rust-native-version846-target').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
for name,cmd in [('journal-and-lease',['cargo','test','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-engine','--lib','mission_','--','--nocapture']),('completion-journal',['cargo','test','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-engine','--lib','agent_completion_journal::tests::','--','--nocapture']),('cli-matrix',['cargo','test','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-cli','--lib','--','--nocapture']),('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-cli','-p','swarm-web','--all-targets','--','-D','warnings']),('rustdoc',['cargo','doc','--manifest-path','rust/Cargo.toml','--offline','-p','swarm-cli','-p','swarm-web','--no-deps'])]:
 start=time.time()
 with (b/(name+'.stdout')).open('x') as out,(b/(name+'.stderr')).open('x') as err:p=subprocess.run(cmd,env=env,stdout=out,stderr=err)
 atomic(b/(name+'.result.json'),json.dumps(dict(argv=cmd,exit=p.returncode,seconds=time.time()-start,global_watchdog=None),indent=2)+'\n');print(name,p.returncode,flush=True)
 if p.returncode:print((b/(name+'.stderr')).read_text()[-4000:],flush=True);break
