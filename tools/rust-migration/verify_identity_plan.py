"""Qualify native plan projection/SQL; keep a crashing Go reference separate."""
import datetime
from collections import Counter
from qualify_identity_plan_v2 import qualify as qualify_v2
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from qualification_inputs import collect
from identity_plan_vectors import vectors

ROOT=Path(__file__).resolve().parents[2];OUTPUT=Path(sys.argv[1]).resolve()
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True)
(ROOT/'.cache').mkdir(exist_ok=True)
WORK=Path(tempfile.mkdtemp(prefix='swarm-plan-contract-',dir=ROOT/'.cache'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')));commands=[]


def save(name,value):
    p=OUTPUT/name;fd,tmp=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
    with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(tmp,p)


def execute(label,args,data=None,expected=0):
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    p=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(raw)
    commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected,observation_only=expected is None));save('commands.json',commands)
    print(label,'exit',p.returncode,flush=True)
    if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
    return p


def run(label,args,data=None):return execute(label,args,data).stdout


def compare(label,raw,go,rust):
    gg=[json.loads(l)for l in run(label+'-go',[str(go)],raw).splitlines()]
    rr=[json.loads(l)for l in run(label+'-rust',[str(rust)],raw).splitlines()]
    inputs=[json.loads(l)for l in raw.splitlines()];assert len(inputs)==len(gg)==len(rr)and inputs
    differences=[dict(input=v,go=g,rust=r)for v,g,r in zip(inputs,gg,rr)if g!=r];save(label+'-differences.json',differences);assert not differences,label+': divergence'
    assert all(g['code']=='ok'for v,g in zip(inputs,gg)if v.get('control',False))
    return dict(cases=len(inputs),valid=sum(g['code']=='ok'for g in gg))


def qualify():
    packages=['./cmd/swarm','./internal/engine',*['./tools/rust-migration/'+p for p in ['plan-layout','plan-oracle','model-layout','work-layout','work-oracle','request-oracle','plan-version-reference','work-row-fixture']]]
    before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages,allow_root=True)
    assert all('rust/crates/swarm-engine/src/'+name in before for name in ['identity_plan_heads.sql','identity_plan_upgrade.sql','identity_plan_upgrade_finish.sql'])
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:
        args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace',*flags,'--all-targets']
        run(label,args)
        run(label+'-inventory',args+['--','--list'])
        expected=Counter(l.removesuffix(': test')for l in (OUTPUT/(label+'-inventory.stdout')).read_text().splitlines()if l.endswith(': test'))
        actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in (OUTPUT/(label+'.stdout')).read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
        assert expected and expected==actual,label+': collected/executed test mismatch'
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
    tests=['EmptyBodyContract','ProjectionBindsContractAndPreservesRuntime','GenerationAndFreshPublication','MigrationAndStampIntegrity','GenerationRollsBackAndRejectsOverflow','CanonicalObjectOrder']
    run('go-plan-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestAgentIdentityPlan('+ '|'.join(tests)+')$','./internal/engine'])
    run('go-vet',['go','vet',*packages])
    layout=json.loads(run('plan-layout',['go','run','./tools/rust-migration/plan-layout']));save('plan-layout.json',layout);(WORK/'plan-layout.json').write_text(json.dumps(layout));generated=WORK/'generated';generated.mkdir()
    run('generate-plan',['python3','tools/rust-migration/generate_identity_plan.py',str(WORK/'plan-layout.json'),str(generated)])
    run('format-plan',['rustfmt','--edition','2021',str(generated/'identity_plan_rules.rs')])
    for name in ['identity_plan_rules.rs','identity_plan_heads.sql']:assert (generated/name).read_bytes()==(ROOT/'rust/crates/swarm-engine/src'/name).read_bytes(),name
    work_layout=run('work-layout',['go','run','./tools/rust-migration/work-layout']);(WORK/'work-layout.json').write_bytes(work_layout)
    run('generate-work',['python3','tools/rust-migration/generate_work_models.py',str(WORK/'work-layout.json'),str(generated/'models.rs')]);run('format-work',['rustfmt','--edition','2021',str(generated/'models.rs')]);assert (generated/'models.rs').read_bytes()==(ROOT/'rust/crates/swarm-engine/src/models.rs').read_bytes()
    request_layout=run('request-layout',['go','run','./tools/rust-migration/model-layout','internal/engine/model.go','internal/engine/task_definition.go']);(WORK/'request-layout.json').write_bytes(request_layout)
    run('generate-request',['python3','tools/rust-migration/generate_request_models.py',str(WORK/'request-layout.json'),str(generated/'request.rs')]);run('format-request',['rustfmt','--edition','2021',str(generated/'request.rs')]);assert (generated/'request.rs').read_bytes()==(ROOT/'rust/crates/swarm-engine/src/request.rs').read_bytes()
    run('native-probes',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--examples'])
    examples=Path(ENV['CARGO_TARGET_DIR'])/'debug/examples';binaries={}
    for name,pkg in [('go-plan','plan-oracle'),('go-work','work-oracle'),('go-request','request-oracle')]:
        binary=WORK/(name+'-bin');run(name+'-build',['go','build','-o',str(binary),'./tools/rust-migration/'+pkg]);binaries[name]=binary
    values=vectors(layout);edge=[v for v in values if v['kind']=='blob'and not v['raw']];ordinary=[v for v in values if not(v['kind']=='blob'and not v['raw'])];assert len(edge)==1
    raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in ordinary)
    plan=compare('plan-corpus',raw,binaries['go-plan'],examples/'identity_plan_probe')
    witness=dict(name='empty-blob-valid-control',kind='blob',raw=b'{}'.hex());probe=[witness,edge[0],dict(witness,name='empty-blob-recovery')];edge_raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in probe)
    native=[json.loads(l)for l in run('native-empty-blob',[str(examples/'identity_plan_probe')],edge_raw).splitlines()];assert len(native)==3 and [v['code']for v in native]==['ok','format','ok']and not native[1]['digest']
    observed=execute('go-empty-blob-observation',[str(binaries['go-plan'])],edge_raw,expected=None)
    if observed.returncode==0:
        gg=[json.loads(l)for l in observed.stdout.splitlines()];assert gg==native;go_edge=dict(status='PASS',paired=True)
    else:
        gg=[json.loads(l)for l in observed.stdout.splitlines()];assert len(gg)==1 and gg[0]==native[0]and observed.stderr
        go_edge=dict(status='OPEN',paired=False,crash_exit=observed.returncode,native_refusal_and_recovery=True,go_direct_contract_test='TestAgentIdentityPlanEmptyBodyContract',owner='Codex / Go-driver maintenance',resumption='source fix or verified driver version; rerun empty-BLOB SQL with recovery',active_go_repaired=False)
    save('go-reference-anomaly.json',go_edge)
    work_raw=(ROOT/'docs/plans/migration-rust/evidence/20261017-13/qualified-02/model-corpus-go.stdin').read_bytes();work=compare('work-regression',work_raw,binaries['go-work'],examples/'work_probe');assert work['cases']==59576
    request_raw=(ROOT/'docs/plans/migration-rust/evidence/20261017-11/qualified-02/go-request-vectors.stdin').read_bytes();request=compare('request-regression',request_raw,binaries['go-request'],examples/'request_probe');assert request['cases']==29662
    v2=qualify_v2(ROOT,OUTPUT,WORK,ENV,run,execute,compare,save,layout,examples,binaries,values)
    # Literal includes are bound before and after, including regenerated SQL.
    close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
    rust=[l for l in (OUTPUT/'rust-tests.stdout').read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')];default=[l for l in (OUTPUT/'rust-default.stdout').read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')]
    events=[json.loads(l)for l in (OUTPUT/'go-plan-tests.stdout').read_text().splitlines()];passed=[v['Test']for v in events if v.get('Action')=='pass'and'Test'in v];assert all('TestAgentIdentityPlan'+name in passed for name in tests)
    hashes={}
    for name,binary in {**binaries,'rust-plan':examples/'identity_plan_probe','rust-work':examples/'work_probe','rust-request':examples/'request_probe'}.items():
        target=OUTPUT/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600);hashes[name]=hashlib.sha256(target.read_bytes()).hexdigest()
    report=dict(status='PARTIAL'if go_edge['status']=='OPEN'else'PASS',scope='native R3c/R3d v1/v2 projection, connection UDFs and transactional head component conversion; not complete Store/migration/authority',v2=v2,native_component_status='PASS',paired_comparisons_status='PASS',go_reference_edge=go_edge,plan=plan,work_regression=work,request_regression=request,source_stable=True,input_graph_closed=True,sql_resource_bound=True,inputs=len(before),registry_packages=len(registry),rust_named_passes=rust,rust_default_named_passes=default,go_named_passes=passed,binary_sha256=hashes,fixture_root=str(WORK),scratch_device=os.stat(WORK).st_dev,scratch_free_bytes=shutil.disk_usage(WORK).free,global_watchdog=None,deployment=False,independent_review=False)
    save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','plan','inputs','registry_packages']}))


try:qualify()
except Exception as error:
    save('failure.json',dict(type=type(error).__name__,cause=str(error),completed_commands=len(commands)));raise
