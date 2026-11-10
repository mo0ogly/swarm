"""Reuse specific original Go results only after exact input/protocol guards."""
import hashlib
import json
from pathlib import Path
from qualification_inputs import stream
from verify_recorded_work import verify_reuse_environment

def production_go_inputs(proof,values):
    """Actual normal/race app/test import closures, including root embeds.

    Additional verification scripts or Rust registry bytes are not Go sources.
    Test variants are explicit roots; dependency TestImports are not compiled.
    """
    root=Path(__file__).resolve().parents[2];goroot=Path(json.loads((proof/'go-env.stdout').read_text())['GOROOT']);keys={'go.mod','go.sum'}
    fields=('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles')
    def key(path):
        if path.is_relative_to(root):return str(path.relative_to(root))
        if path.is_relative_to(goroot):return 'go-toolchain/'+str(path.relative_to(goroot))
        return 'external-go/'+hashlib.sha256(str(path.parent).encode()).hexdigest()+'/'+path.name
    for phase in ['normal','race']:
        packages={p['ImportPath']:p for p in stream((proof/('go-inputs-'+phase+'.stdout')).read_bytes())};targets={root/'cmd/swarm',root/'internal/engine'};pending=[name for name,p in packages.items()if Path(p.get('Dir','/missing'))in targets]
        assert {Path(packages[n]['Dir'])for n in pending}==targets,'actual Go app/test roots missing'
        seen=set()
        while pending:
            name=pending.pop()
            if name in seen:continue
            seen.add(name)
            if name=='C':continue # Go's compiler pseudo-package; Cgo files/CC remain bound.
            assert name in packages,'compiled Go dependency metadata missing: '+name
            p=packages[name];pending.extend(p.get('Imports',[]));directory=Path(p.get('Dir','/missing'))
            for field in fields:
                for source in p.get(field,[]):
                    k=key(directory/source);assert k in values,'Go input not captured: '+k;keys.add(k)
    return{k:values[k]for k in sorted(keys)}

def reuse(baseline,fresh,old,current,corpus):
    verify_reuse_environment(baseline,fresh,old,current)
    assert json.loads((baseline/'go-env.stdout').read_text())==json.loads((fresh/'go-env.stdout').read_text()),'captured Go environment changed'
    original_go=production_go_inputs(baseline,old);fresh_go=production_go_inputs(fresh,current)
    assert original_go==fresh_go,'Go source/normal/race/root/embed closure changed'
    report=json.loads((baseline/'result.json').read_text());assert report['native_component_status']=='PASS'and report['source_stable']
    assert old==json.loads((baseline/'inputs-after.json').read_text()),'original source proof was unstable'
    for name in ['tools/rust-migration/command-reference/main.go','tools/rust-migration/recorded_command_vectors.py','tools/rust-migration/verify_recorded_command.py']:
        assert old[name]==current[name],name+': reference producer changed'
    assert (baseline/'command-corpus-go.stdin').read_bytes()==corpus,'reference corpus changed'
    commands={v['label']:v for v in json.loads((baseline/'commands.json').read_text())}
    names=['go-original-mutation-tests','go-concurrent-writer-race','command-corpus-go','go-vet']
    for name in names:assert commands[name]['exit']==commands[name]['expected_exit']==0,name+': original did not pass'
    original=(baseline/'go-original-mutation-tests.stdout').read_text();race=(baseline/'go-concurrent-writer-race.stdout').read_text()
    assert '[no tests to run]'not in original+race
    for name in report['go_selected_tests']:assert '--- PASS: '+name+' 'in original,name+': test not executed'
    assert '--- PASS: TestConcurrentWriters 'in race
    frames=[json.loads(l)for l in corpus.splitlines()];output=(baseline/'command-corpus-go.stdout').read_bytes();rows=[json.loads(l)for l in output.splitlines()]
    assert len(rows)==len(frames)and rows and all(a['name']==b['Name']for a,b in zip(rows,frames))
    files={name:hashlib.sha256((baseline/name).read_bytes()).hexdigest()for name in ['go-vet.stdout','go-vet.stderr','go-original-mutation-tests.stdout','go-concurrent-writer-race.stdout','command-corpus-go.stdin','command-corpus-go.stdout','result.json','commands.json','inputs-before.json','inputs-after.json','go-env.stdout','go-inputs-normal.stdout','go-inputs-race.stdout']}
    return output,dict(baseline=str(baseline),checks=names,proof_files_sha256=files,go_inputs=len(fresh_go),classification='Reachable cmd/swarm/internal/engine app and test variants, actual normal/race source and embeds',corpus_sha256=hashlib.sha256(corpus).hexdigest(),limits='Local proof reuse, not a signed authority or independent acceptance')
