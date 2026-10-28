"""Reuse R3r compiled-backend checks for frontend-only changes, without weakening guards."""
import hashlib,json
from pathlib import Path
from qualification_inputs import rust_assets
from reuse_go_reference import production_go_inputs
from verify_recorded_work import verify_reuse_environment
from verify_work_engine import reuse_rust_checks
ROOT=Path(__file__).resolve().parents[2]
def reuse(baseline,fresh,current):
    old=json.loads((baseline/'inputs-before.json').read_text());report=json.loads((baseline/'result.json').read_text())
    assert report['native_component_status']=='PASS'and report['source_stable']and old==json.loads((baseline/'inputs-after.json').read_text())
    assets=rust_assets(ROOT,{ROOT/k for k in current if k.startswith('rust/')and k.endswith('.rs')})
    assert all(p.is_relative_to(ROOT/'rust')for p in assets),'Extend native include-closure reuse before embedding outside Rust'
    tests,native=reuse_rust_checks(baseline,fresh,current)
    verify_reuse_environment(baseline,fresh,old,current)
    assert json.loads((baseline/'go-env.stdout').read_text())==json.loads((fresh/'go-env.stdout').read_text())
    previous=production_go_inputs(baseline,old);present=production_go_inputs(fresh,current);assert previous==present,'compiled Go app/test/normal/race/embed inputs changed'
    commands={c['label']:c for c in json.loads((baseline/'commands.json').read_text())};selected=report['go_selected_tests']
    expected={
      'go-reference-tests':['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$'],
      'go-writer-race':['go','test','-race','-timeout=0','-count=1','-v','./internal/engine','-run','^TestConcurrentWriters$'],
      'workspace-compile':['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'],
      'clippy':['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings']}
    proof={}
    for label,args in expected.items():
        c=commands[label];assert c['command']==args and c['exit']==c['expected_exit']==0,label+': not completed or changed'
        for ext in ['stdout','stderr']:name=label+'.'+ext;proof[name]=hashlib.sha256((baseline/name).read_bytes()).hexdigest()
    raw=(baseline/'go-reference-tests.stdout').read_text();race=(baseline/'go-writer-race.stdout').read_text();assert '[no tests to run]'not in raw+race
    for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
    assert '--- PASS: TestConcurrentWriters 'in race
    return dict(status='PASS',scope='Unchanged compiled native and Go backend only; frontend rerun required',baseline=str(baseline),native=native,go_inputs=len(present),native_tests={k:len(v)for k,v in tests.items()},go_selected_tests=selected,proof_files_sha256=proof)
