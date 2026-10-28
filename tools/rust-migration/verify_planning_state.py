"""Source-bound planning/signals/shared date qualification; no global watchdog."""
from collections import Counter
import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
from qualification_inputs import collect
from reuse_go_reference import reuse
from verify_go_reference_reuse import check as check_reuse
from recorded_command_vectors import vectors as prefix_vectors
from mutation_guard_vectors import vectors as guard_vectors
from planning_vectors import vectors
from qualify_planning_state import qualify
ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);work=Path(tempfile.mkdtemp(prefix='swarm-planning-',dir=ROOT/'.cache'))
    # Protected Store fixtures reject group-writable repository ancestors.
    # Create our own 0700 scratch directly below the actual user's safe home;
    # never chmod existing workspace protections or weaken Store validation.
    home=Path.home().resolve();scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-verification-',dir=home));scratch.chmod(0o700)
    for parent in [home,*home.parents]:
        st=parent.stat();assert st.st_uid in [0,os.geteuid()]and(st.st_mode&0o22)==0,'private fixture ancestor is writable by another principal'
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')));commands=[]
    def save(name,value):
        p=output/name;fd,tmp=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,p)
    def execute(label,args,data=None,expected=0):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=env,input=data,capture_output=True)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p
    def run(label,args,data=None):return execute(label,args,data).stdout
    try:
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3j-private-',dir=ROOT/'.cache'));source=run('planning-source',['go','run','./tools/rust-migration/planning-reference']);(prepared/'main.go').write_bytes(source)
        packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/planning-reference','./tools/rust-migration/config-oracle','./'+str(prepared.relative_to(ROOT))]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-04/qualified-02';old=json.loads((baseline/'inputs-before.json').read_text());prefix=prefix_vectors();prefix_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in prefix)
        original_prefix,reused=reuse(baseline,output,old,before,prefix_data);save('reused-go-proof.json',reused);save('reuse-guard-tests.json',check_reuse(baseline,output,old,before,prefix_data,work))
        previous=ROOT/'docs/plans/migration-rust/evidence/20261018-06/qualified-02';prior=json.loads((previous/'inputs-before.json').read_text());assert prior==json.loads((previous/'inputs-after.json').read_text());assert json.loads((previous/'result.json').read_text())['native_component_status']=='PASS'
        changed={'rust/crates/swarm-wire-json/src/lib.rs','rust/crates/swarm-wire-json/Cargo.toml','rust/crates/swarm-enrollment-config/src/go_clock.rs','rust/crates/swarm-engine/src/lib.rs','rust/crates/swarm-engine/Cargo.toml'}
        unchanged=[k for k in prior if k.startswith('rust/')and k not in changed];assert all(before[k]==prior[k]for k in unchanged),'other native contract changed; broaden qualification'
        with tarfile.open(previous/'sources.tar.gz')as archive:clock=archive.extractfile('rust/crates/swarm-enrollment-config/src/go_clock.rs').read().decode()
        extracted=clock[clock.index('fn digits'):clock.index('pub(super) fn deserialize')].replace('pub(super) fn parse','pub fn parse').replace('Error','GoTimestampError').replace('GoTimestampError::Format','GoTimestampError')
        expected=run('shared-clock-extraction-format',['rustfmt','--edition','2021','--emit','stdout'],extracted.encode()).decode()
        actual=(ROOT/'rust/crates/swarm-wire-json/src/go_rfc3339.rs').read_text();assert actual[actual.index('fn digits'):]==expected,'shared date algorithm changed beyond extraction/error adapter'
        save('reused-native-proof.json',dict(baseline=str(previous.relative_to(ROOT)),unchanged=unchanged,shared_date_algorithm_exact_after_declared_error_type_and_format=True,limits='Fresh engine/config/wire checks and config corpus; other leaf data algorithms unchanged'))
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        for label,flags in [('native-all',['--all-features']),('native-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','-p','swarm-wire-json','-p','swarm-enrollment-config',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        selected=['TestPlanningJudgeImportValidatesWithoutPartialMutation','TestJudgeStateHistoryChecksAreDiscriminating','TestHierarchyBudgetsRemainSharedAboveTwentyLevels','TestConfiguredHierarchyLimitsAndCycleGuard','TestPlanningValidationReactivatesAfterResultConsumed','TestPlanningAtomicDecisionReplayAndRestart']
        raw=run('go-planning-tests',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode();assert '[no tests to run]'not in raw
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': test not executed'
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']);run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','-p','swarm-wire-json','-p','swarm-enrollment-config','--all-features','--all-targets','--','-D','warnings']);run('go-new-vet',['go','vet','./tools/rust-migration/planning-reference','./'+str(prepared.relative_to(ROOT))])
        for name in ['planning_state_probe','mutation_guard_probe','recorded_command_probe']:run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples';reference=work/'go-planning';run('planning-reference-build',['go','build','-o',str(reference),str(prepared/'main.go')]);assert source==run('planning-source-after',['go','run','./tools/rust-migration/planning-reference'])
        cases=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in cases);go=[json.loads(l)for l in run('planning-corpus-go',[str(reference)],data).splitlines()];native=[json.loads(l)for l in run('planning-corpus-rust',[str(examples/'planning_state_probe')],data).splitlines()];assert len(go)==len(native)==len(cases)
        differences=[];counterexamples=[]
        for v,g,n in zip(cases,go,native):
            if v['classification']=='parity':
                if g!=n:differences.append(dict(input=v,go=g,rust=n))
            else:
                assert n['code']=='format'and g['body']!=n['body'];assert g['code']==('panic'if v['classification']=='panic'else 'format');counterexamples.append(dict(input=v,go=g,rust=n))
            if v['control']:assert g['code']==n['code']=='ok'
        save('unexpected-differences.json',differences);save('go-signal-counterexamples.json',counterexamples);assert not differences and len(counterexamples)==2
        run('config-oracle-build',['go','build','-o',str(work/'go-config'),'./tools/rust-migration/config-oracle']);run('config-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-config','--example','config_probe']);config=run('config-corpus-go',[str(work/'go-config')]);config_rust=run('config-corpus-rust',[str(examples/'config_probe')],config)
        config_rows=[json.loads(l)for l in config.splitlines()];config_native=[json.loads(l)for l in config_rust.splitlines()];assert config_rows and len(config_rows)==len(config_native);assert all(all(g[k]==n[k]for k in ['name','code','output'])for g,n in zip(config_rows,config_native))
        assert [json.loads(l)for l in run('prefix-regressions',[str(examples/'recorded_command_probe')],prefix_data).splitlines()]==[json.loads(l)for l in original_prefix.splitlines()]
        guard=guard_vectors();guard_data=b''.join(json.dumps(v,ensure_ascii=True,separators=(',',':')).encode()+b'\n'for v in guard);assert guard_data==(previous/'guard-corpus-rust.stdin').read_bytes();assert before['tools/rust-migration/mutation_guard_vectors.py']==prior['tools/rust-migration/mutation_guard_vectors.py'];assert [json.loads(l)for l in run('guard-regressions',[str(examples/'mutation_guard_probe')],guard_data).splitlines()]==[json.loads(l)for l in(previous/'guard-corpus-rust.stdout').read_bytes().splitlines()]
        run('go-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm']);public=qualify(ROOT,output,work,env,run,execute,save,examples/'planning_state_probe')
        for name,binary in [('native-planning-probe',examples/'planning_state_probe'),('go-planning-reference',reference)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3j durable planning, prerequisite/judge validation, staged signals and shared Go clock',cases=len(cases),parity_cases=len(cases)-2,go_signal_counterexamples=2,valid_controls=sum(v['control']for v in cases),config_regressions=len(config_rows),prefix_regressions=len(prefix),guard_regressions=len(guard),tests=tests,go_selected_tests=selected,public_flow=public,source_stable=True,inputs=len(before),registry_packages=len(registry),fixture_root=str(work),global_watchdog=None,full_store=False,public_native_mutation=False,independent_review=False,deployment=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','cases','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
