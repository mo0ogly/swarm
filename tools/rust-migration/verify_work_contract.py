"""Source-bound qualification of the native persisted DTO graph and row envelope."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from qualification_inputs import collect
from work_vectors import vectors
from work_row_scenarios import run as row_scenarios

ROOT=Path(__file__).resolve().parents[2];OUTPUT=Path(sys.argv[1]).resolve()
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True);WORK=Path(tempfile.mkdtemp(prefix='swarm-work-contract-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')))
commands=[]
REUSED={}
RESUME=Path(sys.argv[2]).resolve()if len(sys.argv)>2 else None


def save(path,value):
    p=OUTPUT/path;fd,temporary=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
    with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(temporary,p)


def run(label,args,data=None,expected=0):
    clock=time.monotonic();start=datetime.datetime.now(datetime.timezone.utc).isoformat()
    if label in REUSED:
        prior=REUSED[label];assert prior['command']==args and expected==0 and data is None
        for extension in ['stdout','stderr']:shutil.copyfile(RESUME/(label+'.'+extension),OUTPUT/(label+'.'+extension))
        commands.append(dict(label=label,command=args,executed=False,reference=str(RESUME),original_start_utc=prior['start_utc'],exit=0,expected_exit=0));save('commands.json',commands)
        print(label,'reused successful check under source/environment guards',flush=True)
        return (OUTPUT/(label+'.stdout')).read_bytes()
    p=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    for extension,value in [('stdout',p.stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+extension)).write_bytes(value)
    commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands)
    print(label,'exit',p.returncode,flush=True)
    if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
    return p.stdout


def compare(label,raw,go,rust,controls=False):
    g=run(label+'-go',[str(go)],raw);r=run(label+'-rust',[str(rust)],raw)
    inputs=[json.loads(l)for l in raw.splitlines()];gg=[json.loads(l)for l in g.splitlines()];rr=[json.loads(l)for l in r.splitlines()]
    assert len(inputs)==len(gg)==len(rr)and inputs
    assert all(v['name']==x['name']==y['name']for v,x,y in zip(inputs,gg,rr))
    differences=[dict(input=v,go=x,rust=y)for v,x,y in zip(inputs,gg,rr)if x!=y]
    save(label+'-differences.json',differences);assert not differences,label+': divergence'
    if controls:assert all(x['code']=='ok'for v,x in zip(inputs,gg)if v['control'])
    return dict(cases=len(inputs),valid=sum(x['code']=='ok'for x in gg),controls=sum(v.get('control',False)for v in inputs))


def deep_vectors():
    values=[]
    for depth in [1,64,512,1024,2048,9999,10000,10001]:
        if depth==10001:values.append(dict(name='depth-valid-control',model='ManagedFragmentJournalAnchor',raw=b'{}'.hex()))
        raw=b'{"replanned_from":'*(depth-1)+b'{}'+b'}'*(depth-1)
        values.append(dict(name='anchor-depth-'+str(depth),model='ManagedFragmentJournalAnchor',raw=raw.hex()))
    for depth in [9999,10000]:
        values.append(dict(name='raw-valid-control-'+str(depth),model='Event',raw=b'{}'.hex()))
        raw=b'{"payload":'+b'['*depth+b'null'+b']'*depth+b'}'
        values.append(dict(name='opaque-depth-'+str(depth),model='Event',raw=raw.hex()))
    for i,raw in enumerate([
        b'{"replanned_from":{"plan":"old","journal":"first"},"replanned_from":{"journal":"new"}}',
        b'{"replanned_from":{"plan":"old"},"replanned_from":null,"replanned_from":{}}',
        b'{"replanned_from":'+b'{"replanned_from":'*2048+b'false'+b'}'*2048+b'}',
    ]):
        values.append(dict(name='anchor-merge-control-'+str(i),model='ManagedFragmentJournalAnchor',raw=b'{}'.hex()))
        values.append(dict(name='anchor-merge-'+str(i),model='ManagedFragmentJournalAnchor',raw=raw.hex()))
    return b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in values)


def default_guard():
    client=WORK/'default-consumer';(client/'src').mkdir(parents=True)
    manifest='[package]\nname="work-default-consumer"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nswarm-engine={path='+json.dumps(str(ROOT/'rust/crates/swarm-engine'))+'}\n'
    (client/'Cargo.toml').write_text(manifest);shutil.copyfile(ROOT/'rust/Cargo.lock',client/'Cargo.lock')
    control='fn main(){let _=swarm_engine::decode_work_row("fixture",1,b"{}");}\n'
    (client/'src/main.rs').write_text(control)
    run('consumer-lock',['cargo','generate-lockfile','--offline','--manifest-path',str(client/'Cargo.toml')])
    main={(p['name'],p['version']):p.get('checksum')for p in tomllib.loads((ROOT/'rust/Cargo.lock').read_text())['package']}
    assert all(main.get((p['name'],p['version']))==p['checksum']for p in tomllib.loads((client/'Cargo.lock').read_text())['package']if p.get('source'))
    args=['cargo','check','--locked','--offline','--manifest-path',str(client/'Cargo.toml')]
    run('consumer-valid-control',args)
    (client/'src/main.rs').write_text('fn main(){let _=swarm_engine::fixture_model_roundtrip;}\n')
    run('consumer-fixture-refusal',args,expected=101)
    assert 'E0425'in (OUTPUT/'consumer-fixture-refusal.stderr').read_text()
    for name in ['Cargo.toml','Cargo.lock']:shutil.copyfile(client/name,OUTPUT/('consumer-'+name))
    return dict(control_compiled=True,fixture_api_refused=True,expected_code='E0425',registry_checksums_match=True)


def qualify():
    packages=['./cmd/swarm','./internal/engine',*['./tools/rust-migration/'+p for p in ['work-layout','work-oracle','work-row-fixture','request-oracle']]]
    before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages,allow_root=True)
    if RESUME:
        original=json.loads((RESUME/'inputs-before.json').read_text());assert set(original)==set(before)
        assert all(before[k]==v for k,v in original.items()if k!='tools/rust-migration/verify_work_contract.py'), 'prior tested inputs changed'
        for label in ['go-env','go-version','rust-version','cc-version','build-environment.json']:
            filename=label if label.endswith('.json')else label+'.stdout'
            assert (RESUME/filename).read_bytes()==(OUTPUT/filename).read_bytes(), 'prior environment changed'
        prior={v['label']:v for v in json.loads((RESUME/'commands.json').read_text())}
        for label in ['rust-format','rust-tests','rust-default','rust-clippy','go-row-tests','go-vet','ar-version']:
            entry=prior[label];assert entry['exit']==entry['expected_exit']==0 and entry.get('executed',True)
            REUSED[label]=entry
        save('reused-checks.json',dict(reference=str(RESUME),checks=list(REUSED),guarded_inputs=len(before)-1,only_changed_verification_tool='verify_work_contract.py',overall_prior_attempt='incomplete; never reused as global PASS'))
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:
        run(label,['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace',*flags,'--all-targets'])
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
    names=['StoreGetRejectsInvalidRows','StoreListRejectsPartialResult','CommonMutationRejectsBeforeCallbacks','PublicReadersRefuseIncoherentWork','ReplayReturnsCurrentConsistentState','ReadEnvelopePreservesLegacyAndCauses']
    run('go-row-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestWorkRow('+ '|'.join(names)+')$','./internal/engine'])
    run('go-vet',['go','vet',*packages]);run('ar-version',['ar','--version'])
    layout=json.loads(run('work-layout',['go','run','./tools/rust-migration/work-layout']));assert len(layout['types'])==85 and sum(len(t['fields'])for t in layout['types'].values())==771
    (WORK/'layout.json').write_text(json.dumps(layout));generated=WORK/'models.rs'
    run('generate-models',['python3','tools/rust-migration/generate_work_models.py',str(WORK/'layout.json'),str(generated)])
    run('format-models',['rustfmt','--edition','2021',str(generated)]);assert generated.read_bytes()==(ROOT/'rust/crates/swarm-engine/src/models.rs').read_bytes()
    run('rust-probes',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--examples'])
    examples=Path(ENV['CARGO_TARGET_DIR'])/'debug/examples';binaries={}
    for name,pkg in [('go-model','./tools/rust-migration/work-oracle'),('go-setter','./tools/rust-migration/work-row-fixture'),('go-cli','./cmd/swarm')]:
        file=WORK/(name+'-bin');run(name+'-build',['go','build','-o',str(file),pkg]);binaries[name]=file
    vv=vectors(layout);raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in vv)
    compared=compare('model-corpus',raw,binaries['go-model'],examples/'work_probe',True)
    deep=compare('deep-corpus',deep_vectors(),binaries['go-model'],examples/'work_probe')
    rows=row_scenarios(binaries['go-cli'],examples/'work_probe',binaries['go-setter'],WORK,OUTPUT,ENV)
    print('row scenarios',rows['operations'],flush=True)
    reference=ROOT/'docs/plans/migration-rust/evidence/20261017-11/qualified-02';old=json.loads((reference/'inputs-before.json').read_text())
    assert json.loads((reference/'result.json').read_text())['status']=='PASS'
    go_keys=[k for k in old if k in ['go.mod','go.sum']or k.endswith('.go')and not k.startswith(('registry/','rust/'))]
    assert go_keys and all(before.get(k)==old[k]for k in go_keys), 'Go reference inputs changed'
    assert (reference/'go-version.stdout').read_bytes()==(OUTPUT/'go-version.stdout').read_bytes()
    assert json.loads((reference/'go-env.stdout').read_text())==json.loads((OUTPUT/'go-env.stdout').read_text())
    raw=(reference/'rust-request-vectors.stdin').read_bytes();native=run('request-regression',[str(examples/'request_probe')],raw)
    expected=(reference/'go-request-vectors.stdout').read_bytes();assert [json.loads(l)for l in native.splitlines()]==[json.loads(l)for l in expected.splitlines()]
    request_cases=len(native.splitlines());assert request_cases==29662
    guard=default_guard();tree=run('default-engine-features',['cargo','tree','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','-e','features']);assert b'test-fixtures'not in tree
    close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
    rust_names=[l for l in (OUTPUT/'rust-tests.stdout').read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')]
    default_names=[l for l in (OUTPUT/'rust-default.stdout').read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')]
    assert len(rust_names)==111 and len(default_names)==100,(len(rust_names),len(default_names))
    events=[json.loads(l)for l in (OUTPUT/'go-row-tests.stdout').read_text().splitlines()];passed=[v['Test']for v in events if v.get('Action')=='pass'and'Test'in v]
    assert all('TestWorkRow'+n in passed for n in names)
    hashes={}
    for name,file in {**binaries,'rust-model':examples/'work_probe','rust-request':examples/'request_probe'}.items():
        destination=OUTPUT/(name+'.binary');shutil.copyfile(file,destination);destination.chmod(0o600);hashes[name]=hashlib.sha256(destination.read_bytes()).hexdigest()
    report=dict(status='PASS',scope='R3b UTF-8 persisted DTO graph and row codec; native SQL/mutations/complete engine remain open',source_stable=True,input_graph_closed=True,inputs=len(before),registry_packages=len(registry),generated_types=83,shared_types=2,go_fields=771,model=compared,deep=deep,rows=rows,request_regression_cases=request_cases,rust_named_passes=rust_names,rust_default_named_passes=default_names,go_named_passes=passed,default_feature_guard=guard,binary_sha256=hashes,fixture_root=str(WORK),global_watchdog=None,deployment=False,independent_review=False)
    save('result.json',report);print(json.dumps({k:report[k]for k in ['status','inputs','registry_packages','model','deep','request_regression_cases']}))


try:qualify()
except Exception as error:
    save('failure.json',dict(type=type(error).__name__,cause=str(error),completed_commands=len(commands)));raise
