#!/usr/bin/env python3
"""Compare actual Go/Rust numeric budgets on isolated recorded fixtures.

No provider, live database, claim, lease, identity or financial reservation runs.
Builds separate fixture and production profiles; output must be fresh. No timeout,
retry or overwrite. PASS covers numeric snapshots only, not transaction admission.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]


def fresh(path, value):
    """Write immutable fixture evidence exclusively and sync; preserve failures."""
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + '\n').encode()
    with path.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def verify(output):
    """Build probes and compare complete observations after valid witnesses.

    Explicit production profile excludes fixture features. Original commands,
    binary hashes, source copies and output survive failures. Invalid inputs are
    refusal data, not a successful zero count; no mutable store is touched.
    """
    output.mkdir(mode=0o700)
    commands, checks = [], []
    env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/'rust/target/r25'),
               CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_INCREMENTAL='0', RUSTDOCFLAGS='-D warnings')
    cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
    def run(label, args, data=None, extra=None):
        """Record one finite fixture command; preserve and propagate any failure."""
        args = list(map(str, args))
        result = capture(args, cwd=ROOT, env=dict(env, **(extra or {})), data=data,
                         stdout_path=output/(label+'.stdout'), stderr_path=output/(label+'.stderr'))
        record = dict(label=label, args=args, exit=result.returncode)
        commands.append(record)
        fresh(output/(label+'.command.json'), record)
        print(label, result.returncode, flush=True)
        if result.returncode:
            raise RuntimeError(label+': unexpected exit, original outputs retained')
        return result.stdout
    fresh(output/'harness.py', Path(__file__).read_bytes())
    sources = ['rust/crates/swarm-engine/src/planning_budget.rs',
               'rust/crates/swarm-engine/examples/planning_budget_probe.rs',
               'internal/engine/planning_budget.go',
               'internal/engine/rust_migration_planning_budget_probe_test.go']
    for number, name in enumerate(sources):
        fresh(output/f'source-{number}.txt', (ROOT/name).read_bytes())
    fixture_binary = output/'native-budget-fixture'
    production = output/'native-production-cli'
    go = output/'go-planning'
    run('fixture-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine', '--features', 'test-fixtures', '--example', 'planning_budget_probe'])
    shutil.copyfile(ROOT/'rust/target/r25/debug/examples/planning_budget_probe', fixture_binary)
    fixture_binary.chmod(0o700)
    run('production-build', ['cargo', 'build', *cargo, '-p', 'swarm-cli', '--bin', 'swarm-rust'])
    shutil.copyfile(ROOT/'rust/target/r25/debug/swarm-rust', production)
    production.chmod(0o700)
    tree = run('production-features', ['cargo', 'tree', *cargo, '-p', 'swarm-cli', '--format', '{p} features=[{f}]', '--edges', 'normal'])
    if b'test-fixtures' in tree:
        raise ValueError('production-profile-includes-fixtures')
    run('go-build', ['go', 'test', '-c', '-o', go, './internal/engine'])
    private = output/'fixture'
    private.mkdir(mode=0o700)
    fresh(private/'.rust-migration-fixture', b'Owned numeric budget fixture\n')
    base = dict(schema_version=1, id='work', revision=1, criteria=['Child', 'Other'], tasks=[
        dict(id='task-child', scope_id='child', requirements=['req-1'], status='accepted'),
        dict(id='task-other', scope_id='other', requirements=['req-2'], status='failed')],
        planning=dict(version=1, max_tasks=10, max_decisions=10, max_activations=20,
                      scopes=[dict(id='root', revision=1), dict(id='parent', parent='root', revision=1, max_tasks=2, max_activations=2),
                              dict(id='child', parent='parent', revision=1, requirements=['req-1']),
                              dict(id='other', parent='root', revision=1, requirements=['req-2'])]))
    def case(label, work=None, scope='child', refused=False):
        """Run both actual producers and assert exact data, not merely exit zero."""
        work = copy.deepcopy(base if work is None else work)
        frame, result = output/(label+'.frame.json'), output/(label+'.go.json')
        fresh(frame, dict(Root=str(private), Output=str(result), Scope=scope, Work=work))
        raw = run(label+'-go', [go, '-test.run', '^TestRustMigrationPlanningBudgetProbe$', '-test.v', '-test.timeout=0'], extra={'SWARM_PLANNING_BUDGET_PROBE':str(frame)})
        if b'--- PASS: TestRustMigrationPlanningBudgetProbe' not in raw or b'SKIP' in raw:
            raise ValueError('go-probe-not-executed')
        request = (json.dumps(dict(work=work, scope=scope))+'\n').encode()
        fresh(output/(label+'.native-input.jsonl'), request)
        actual = json.loads(run(label+'-native', [fixture_binary], request))
        expected = json.loads(result.read_bytes())
        if actual != expected or actual['code'] != ('refused' if refused else 'ok'):
            raise ValueError(label+': producer observations differ')
        checks.append(dict(name=label, observation=actual))
        return actual
    for scope, ancestors, count in [('child',['child','parent','root'],1),('parent',['parent','root'],1),('root',['root'],2),('other',['other','root'],1)]:
        actual = case('valid-'+scope, scope=scope)
        assert actual['ancestors']==ancestors and actual['task_count']==count and actual['activation_budget_available'] and actual['task_budget_available']
    for key, limit in [('activations',20),('decisions',10)]:
        for used in (limit-1,limit,limit+1):
            case(key+'-'+str(used)+'-witness')
            work=copy.deepcopy(base);work['planning'][key]=used
            actual=case(key+'-'+str(used),work,refused=used>limit)
            if used<=limit:
                assert actual['activation_budget_available']==(used<limit)
    for used in (1,2,3):
        case('ancestor-'+str(used)+'-witness')
        work=copy.deepcopy(base);work['planning']['activations']=used
        work['planning']['scopes'][0]['activations']=used
        work['planning']['scopes'][1]['activations']=used
        assert case('ancestor-'+str(used),work)['activation_budget_available']==(used<2)
        assert case('sibling-'+str(used),work,'other')['activation_budget_available']
    work=copy.deepcopy(base);task=copy.deepcopy(work['tasks'][0]);task['id']='another-child';work['tasks'].append(task)
    case('task-equality-witness')
    assert not case('task-equality',work)['task_budget_available']
    assert case('task-sibling',work,'other')['task_budget_available']
    for limit in (1,2,3):
        case('global-task-'+str(limit)+'-witness')
        work=copy.deepcopy(base);work['planning']['max_tasks']=limit
        actual=case('global-task-'+str(limit),work,refused=limit<2)
        if limit>=2:
            assert actual['task_budget_available']==(limit>2)
    for name in ('unknown','empty','missing','cycle','negative'):
        case(name+'-witness')
        work=copy.deepcopy(base);scope='child'
        if name=='unknown':scope='PRIVATE_UNKNOWN'
        if name=='empty':scope=''
        if name=='missing':work.pop('planning')
        if name=='cycle':work['planning']['scopes'][1]['parent']='child'
        if name=='negative':work['planning']['scopes'][1]['activations']=-1
        case(name,work,scope,True)
        case(name+'-recovery')
    hashes = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [go,fixture_binary,production]}
    fresh(output/'result.json', dict(status='PASS', count=len(checks), checks=checks, binaries=hashes,
          scope='Actual Go/native numeric observations; not full claim, rights, reservation or installation', commands=commands))
    print('PASS',len(checks),'Go/native numeric budget comparisons',flush=True)


def main():
    """Accept a fresh output or create a private temporary one; errors stay nonzero."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    if args.output is None:
        parent=Path(tempfile.mkdtemp(prefix='swarm-budget-proof-'))
        output=parent/'evidence'
    else:
        output=args.output.resolve()
    print('Evidence:',output,flush=True)
    verify(output)


if __name__=='__main__':
    main()
