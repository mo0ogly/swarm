"""Finish source-bound Work/Admin checks after a diagnosed recipe error.

Product test results are reused only with identical effective product inputs;
original failure remains FAIL. No source mutation, provider or deployment occurs.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import time
from collections import Counter
from qualification_inputs import collect
from recorded_process import capture
ROOT=Path(__file__).resolve().parents[2]

def main():
    """Prove the precise tool-only delta, execute missing checks and bind evidence."""
    if not __debug__ or len(sys.argv)!=3:
        raise RuntimeError('active assertions, failed evidence and fresh destination required')
    prior=Path(sys.argv[1]).resolve();output=Path(sys.argv[2]).resolve()
    output.mkdir(parents=True,exist_ok=False);env=dict(os.environ);commands=[]
    def save(name,value):
        """Atomically publish final JSON records without destroying original evidence."""
        destination=output/name;fd,temporary=tempfile.mkstemp(dir=output,prefix='.'+name)
        with os.fdopen(fd,'w')as stream:
            json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())
        os.replace(temporary,destination)
    def run(label,args,data=None):
        """Stream complete original outputs without a global watchdog or retries."""
        args=list(map(str,args));started=datetime.datetime.now(datetime.timezone.utc).isoformat();start=time.monotonic()
        result=capture(args,cwd=ROOT,env=env,data=data,stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        commands.append(dict(label=label,command=args,start_utc=started,seconds=time.monotonic()-start,exit=result.returncode));save('commands.json',commands)
        print(label,'exit',result.returncode,flush=True)
        if result.returncode:raise RuntimeError(label+': original failure retained')
        return result.stdout
    try:
        previous=json.loads((prior/'commands.json').read_text())
        assert previous[-1]['label']=='i18n' and previous[-1]['exit']==1
        assert previous[-1]['command']==['npm','run','i18n:check']
        assert all(c['exit']==0 for c in previous[:-1])
        assert 'Missing script: "i18n:check"'in(prior/'i18n.stderr').read_text()
        old_inputs=json.loads((prior/'inputs-before.json').read_text())
        with tarfile.open(prior/'sources.tar.gz')as archive:
            original=archive.extractfile('tools/rust-migration/verify_work_view.py').read()
        key='tools/rust-migration/verify_work_view.py'
        assert hashlib.sha256(original).hexdigest()==old_inputs[key]
        old=b"        run('i18n',['npm','run','i18n:check'])"
        new=b"        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check'])\n        run('i18n',['npm','run','test:i18n'])"
        assert original.count(old)==1 and (ROOT/key).read_bytes()==original.replace(old,new)
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine'],allow_root=True)
        changed={k for k in set(before)|set(old_inputs)if before.get(k)!=old_inputs.get(k)}
        assert changed=={key,'tools/rust-migration/finish_work_view.py'},changed
        actual=Counter(line[5:-7]for line in(prior/'workspace-tests.stdout').read_text().splitlines()if line.startswith('test ')and line.endswith(' ... ok'))
        listed=Counter(line[:-6]for line in(prior/'workspace-inventory.stdout').read_text().splitlines()if line.endswith(': test'))
        assert actual and actual==listed==Counter(json.loads((prior/'workspace-test-names.json').read_text()))
        comparisons=json.loads((prior/'public-comparisons.json').read_text());assert len(comparisons)==52
        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check'])
        run('i18n-tests',['npm','run','test:i18n'])
        run('methods',['python3','tools/agent-workflows/check.py'])
        run('diff-check',['git','diff','--check'])
        save('reused-evidence.json',dict(directory=str(prior),original_status='FAIL',product_inputs_unchanged=True,tool_only_delta=sorted(changed),verified_test_names=list(actual.elements()),public_comparisons=comparisons,reused_checks=[c['label']for c in previous[:-1]]))
        target=Path(env['CARGO_TARGET_DIR'])/'debug'
        reference=Path(next(c['command'][4]for c in previous if c['label']=='go-build'))
        # go build -o PATH ./cmd/swarm: verify the exact original argument position.
        build=next(c['command']for c in previous if c['label']=='go-build');assert build[:3]==['go','build','-o'];reference=Path(build[3])
        binaries={}
        for name,source in [('go-cli',reference),('native-cli',target/'swarm-rust'),('projection-probe',target/'examples/work_projection_probe')]:
            destination=output/(name+'.binary');shutil.copyfile(source,destination);destination.chmod(0o600);binaries[name]=hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json',binaries)
        close_graphs();after=snapshot();assert before==after,'effective inputs changed during finishing checks';save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',scope='R5e51 public Work views and basic mutations; full migration open',workspace_tests=sum(actual.values()),public_comparisons=len(comparisons),reused_from=str(prior),product_inputs_unchanged=True,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,provider_calls=False,independent_review=False,deployment=False))
        print('PASS R5e51 component; PARTIAL full migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise

def finish_phase_ui(prior, output):
    """Finish R3ax54 after correcting only its quality-provenance test assertion.

    Requires identical effective compiled/frontend inputs, with exactly the known
    recipe delta and this finisher changed. Revalidates named native/Go evidence,
    original public comparisons, actual toolchain/environment and cached binaries;
    reruns all browser journeys and missing checks. No provider/server operation
    or overall migration acceptance follows. Every failure retains its originals;
    shared source review remains mandatory before publishing a component pass.
    """
    import re
    from verify_work_view import reuse_go_checks
    from verify_admin_programs import verify_runtime_archive
    output.mkdir(parents=True, exist_ok=False)
    env=dict(os.environ);commands=[]
    def save(name, value):
        """Atomically save owned records, preserving historical source evidence."""
        path=output/name;fd,tmp=tempfile.mkstemp(dir=output,prefix='.'+name)
        with os.fdopen(fd,'w')as stream:
            json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())
        os.replace(tmp,path)
    def run(label,args,data=None):
        """Stream original command output with no global watchdog, shell or retry."""
        args=list(map(str,args));start=time.monotonic();began=datetime.datetime.now(datetime.timezone.utc).isoformat()
        result=capture(args,cwd=ROOT,env=env,data=data,stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        commands.append(dict(label=label,command=args,start_utc=began,seconds=time.monotonic()-start,exit=result.returncode));save('commands.json',commands)
        print(label,'exit',result.returncode,flush=True)
        if result.returncode:raise RuntimeError(label+': original failure retained')
        return result.stdout
    try:
        previous=json.loads((prior/'commands.json').read_bytes());by_label={c['label']:c for c in previous}
        assert previous[-1]['label']=='admin'and previous[-1]['exit']==1
        assert all(c['exit']==c['expected_exit']for c in previous[:-1])
        failure=(prior/'admin.stderr').read_text();assert 'settings_revision' in failure and 'deepStrictEqual'in failure
        old=json.loads((prior/'inputs-before.json').read_bytes());recipe='tests/rust_engine_settings_admin_ui.cjs'
        with tarfile.open(prior/'sources.tar.gz')as archive:original=archive.extractfile(recipe).read()
        assert hashlib.sha256(original).hexdigest()==old[recipe]
        old_line=b"  assert.deepEqual(await cli(root,['engine-settings','quality-profile']),phaseQuality);"
        new_line=b"  const afterPhases=await cli(root,['engine-settings','quality-profile']);assert(BigInt(afterPhases.settings_revision)>BigInt(phaseQuality.settings_revision));assert.deepEqual(afterPhases,{...phaseQuality,settings_revision:afterPhases.settings_revision});"
        expected=original.replace(old_line,new_line)
        replacements=[
            (b"  const phaseInitial=(await cli(root,['work','list'])).find(w=>w.id===wid);const oldMappings=phaseInitial.tasks.map(t=>({id:t.id,checks:t.plan_checks}));", b"  const originalBeforePhases=(await cli(root,['work','list'])).find(w=>w.id===wid);\n  const phaseBase=await mutate(root,'','work.create',req('phase-work-'+label,0,{title:'Isolated phase consumer',objective:'Configured defaults',scope:'Owned fixture',criteria:['Proof']}));assert.equal(phaseBase.code,'ok');const phaseWid=phaseBase.work.id;\n  const phaseWitness=await mutate(root,phaseWid,'task.add',req('phase-witness-'+label,phaseBase.work.revision,{id:'phase-before',title:'Prior contract',deliverable:'Report',criteria:['Proof']}));assert.equal(phaseWitness.code,'ok');assert.equal(Object.keys(phaseWitness.work.tasks[0].plan_checks).length,4);\n  const phaseInitial=phaseWitness.work;const oldMappings=phaseInitial.tasks.map(t=>({id:t.id,checks:t.plan_checks}));"),
            (b"mutate(root,wid,'task.add',phaseRequest)", b"mutate(root,phaseWid,'task.add',phaseRequest)"),
            (b"mutate(root,wid,'task.add',req('phase-empty-'", b"mutate(root,phaseWid,'task.add',req('phase-empty-'"),
            (b"mutate(root,wid,'task.update',req('phase-title-'", b"mutate(root,phaseWid,'task.update',req('phase-title-'"),
            (b"  const afterPhases=await cli(root,['engine-settings','quality-profile']);assert(BigInt(afterPhases.settings_revision)>BigInt(phaseQuality.settings_revision));assert.deepEqual(afterPhases,{...phaseQuality,settings_revision:afterPhases.settings_revision});", b"  const afterPhases=await cli(root,['engine-settings','quality-profile']);assert(BigInt(afterPhases.settings_revision)>BigInt(phaseQuality.settings_revision));assert.deepEqual(afterPhases,{...phaseQuality,settings_revision:afterPhases.settings_revision});\n  assert.deepEqual((await cli(root,['work','list'])).find(w=>w.id===wid),originalBeforePhases);"),
        ]
        for left,right in replacements:
            assert expected.count(left)==1
            expected=expected.replace(left,right)
        assert original.count(old_line)==1 and (ROOT/recipe).read_bytes()==expected
        extra=[ROOT/p for p in ['frontend/engine-settings/admin.js','frontend/engine-settings/admin.css',recipe,'tests/helpers/browser_verification_runtime.cjs','locales/en.json','web/i18n-en.js','package-lock.json']]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine'],allow_root=True,extra_files=extra)
        changed={k for k in set(before)|set(old)if before.get(k)!=old.get(k)}
        assert changed=={recipe,'tools/rust-migration/finish_work_view.py'},changed
        assert json.loads((prior/'build-environment.json').read_bytes())==json.loads((output/'build-environment.json').read_bytes())
        for label in ['compile','clippy','workspace-tests','workspace-inventory','review-guard-tests']:
            assert by_label[label]['exit']==by_label[label]['expected_exit']==0
        actual=Counter(line[5:-7]for line in(prior/'workspace-tests.stdout').read_text().splitlines()if line.startswith('test ')and line.endswith(' ... ok'))
        listed=Counter(line[:-6]for line in(prior/'workspace-inventory.stdout').read_text().splitlines()if line.endswith(': test'))
        assert actual and actual==listed==Counter(json.loads((prior/'workspace-test-names.json').read_bytes()))
        go_prior=Path(json.loads((prior/'reused-go-checks.json').read_bytes())['directory'])
        go_commands=json.loads((go_prior/'commands.json').read_bytes());pattern=next(c['command'][-1]for c in go_commands if c['label']=='go-targeted')
        go_names=reuse_go_checks(go_prior,output,before,pattern)
        comparisons=[];gates=[]
        for label in by_label:
            if not re.search(r'-(fr|en)-(list|show)-(json|text)-go$',label):continue
            peer=label[:-3]+'-native';assert by_label[label]['exit']==by_label[peer]['exit']==0
            left=(prior/(label+'.stdout')).read_bytes();right=(prior/(peer+'.stdout')).read_bytes()
            assert json.loads(left)==json.loads(right)if '-json-'in label else left==right
            comparisons.append(dict(label=label,go_sha256=hashlib.sha256(left).hexdigest(),native_sha256=hashlib.sha256(right).hexdigest()))
        assert len(comparisons)==60
        for state in ['pass','denied']:
            for lang in ['fr','en']:
                left='gate-'+state+'-go-replay-'+lang;right='gate-'+state+'-native-replay-'+lang
                assert by_label[left]['exit']==by_label[right]['exit']==(0 if state=='pass'else 1)
                assert json.loads((prior/(left+'.stdout')).read_bytes())==json.loads((prior/(right+'.stdout')).read_bytes())
                gates.append(dict(state=state,language=lang))
        save('reused-native-public.json',dict(original_directory=str(prior),original_status='FAIL',scope='Source-identical native checks/public comparisons; corrected UI rerun required',native_tests=sum(actual.values()),public_comparisons=comparisons,gate_comparisons=gates,unchanged_compiled_inputs=True,changed_method_inputs=sorted(changed),proof_files={label+'.'+ext:hashlib.sha256((prior/(label+'.'+ext)).read_bytes()).hexdigest()for label in ['workspace-tests','workspace-inventory','compile','clippy']for ext in ['stdout','stderr']}))
        cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline'];target=Path(env['CARGO_TARGET_DIR'])/'debug';native=target/'swarm-rust'
        probes=[target/'examples'/name for name in ['root_store_probe','project_context_probe','git_report_probe','baseline_files_probe','review_reply_probe','work_projection_probe']]
        artifacts={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in [native,*probes]}
        run('cli-cached-build',['cargo','build',*cargo,'-p','swarm-cli']);run('probes-cached-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--examples'])
        assert artifacts=={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in[native,*probes]},'cached artifact changed'
        save('cached-binaries-before-after.json',artifacts)
        run('browser-runtime-guards',['node','tests/browser_verification_runtime_test.cjs'])
        run('admin',['node',recipe,native,probes[0],output/'admin',*probes[1:]])
        save('browser-runtime-archive.json',verify_runtime_archive(output/'admin'))
        ui=json.loads((output/'admin/result.json').read_bytes());assert ui['status']=='PASS'and len(ui['checks'])==4 and not ui['errors']
        assert all(c['saved_admin_manual_phases_change_new_tasks']and c['manual_phases_empty_and_existing_contract_preserved']and c['malformed_phase_transport_refused_without_mutation']for c in ui['checks'])
        run('go-vet',['go','vet','./cmd/swarm','./internal/engine'])
        run('rustdoc',['cargo','doc',*cargo,'--workspace','--all-features','--no-deps'])
        run('doctests',['cargo','test',*cargo,'--workspace','--all-features','--doc'])
        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check']);run('i18n',['npm','run','test:i18n'])
        run('methods',['python3','tools/agent-workflows/check.py']);run('diff-check',['git','diff','--check'])
        reference=Path(by_label['go-build']['command'][3]);assert reference.is_relative_to(ROOT/'.cache')
        binaries={}
        for name,source in [('go-cli',reference),('native-cli',native),*[(p.name,p)for p in probes]]:
            dest=output/(name+'.binary');shutil.copyfile(source,dest);dest.chmod(0o600);binaries[name]=hashlib.sha256(dest.read_bytes()).hexdigest()
        save('binaries.json',binaries)
        close_graphs();after=snapshot();assert before==after,'inputs changed during UI completion';save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',scope='R3ax54 persisted manual phase defaults; functional Admin candidate',native_tests=sum(actual.values()),native_checks_reused_from=str(prior),go_tests=len(go_names),go_checks_reused_from=str(go_prior),fresh_backend_tests=0,admin_checks=ui['checks'],public_comparisons=len(comparisons),gate_comparisons=len(gates),inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,provider_calls=False,independent_review=False,deployment=False,admin_installed=False,visual_acceptance='requires image inspection'))
        print('PASS R3ax54 component; PARTIAL migration; backend proofs reused, Admin rerun',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise

if __name__=='__main__':
    if len(sys.argv)==4 and sys.argv[3]=='phases-ui':
        finish_phase_ui(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())
    else:main()
