"""Compile once; run every discovered Go test in disjoint bounded processes."""
import concurrent.futures
import json
import re
import subprocess
import tempfile
import time
from pathlib import Path


def partition(names, plan):
    if not names or len(names) != len(set(names)):
        raise ValueError('empty or duplicate test inventory')
    isolated = plan['isolated']
    dedicated = plan.get('dedicated', [])
    reserved = isolated + dedicated
    if len(reserved) != len(set(reserved)) or not set(reserved) <= set(names):
        raise ValueError('isolated test inventory mismatch')
    remaining = set(names) - set(reserved)
    groups = [[] for _ in range(min(plan['groups'], len(remaining)))]
    weights = plan['observed_seconds']
    costs = [0.0 for _ in groups]
    # Greedy scheduling changes order only. Unknown durations have a conservative
    # unit cost; observed durations under load are hints, never acceptance data.
    for name in sorted(remaining, key=lambda n: (-weights.get(n, 1), n)):
        index = min(range(len(groups)), key=lambda i: (costs[i], len(groups[i]), i))
        groups[index].append(name)
        costs[index] += weights.get(name, 1)
    # Long protocols get their own bounded process. They are not timing-sensitive
    # isolated cases, so run them alongside the remaining distributed groups.
    all_groups = [[name] for name in reserved] + groups
    if sorted(name for group in all_groups for name in group) != sorted(names):
        raise ValueError('incomplete or duplicate coverage')
    return all_groups, len(isolated)


def run(group, index, binary, package, timeout):
    pattern = '^(?:' + '|'.join(re.escape(name) for name in group) + ')$'
    command = ['go', 'tool', 'test2json', '-t', '-p', package, str(binary),
               '-test.run', pattern, '-test.count=1', '-test.v=test2json',
               f'-test.timeout={timeout}s']
    started = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    observed, passed, skipped = set(), set(), {}
    last = started
    failures = []
    for line in process.stdout:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            failures.append(line.rstrip())
            continue
        test, action = event.get('Test', ''), event.get('Action')
        if action == 'run' and '/' not in test:
            observed.add(test)
        if action == 'pass' and test and '/' not in test:
            passed.add(test)
        if action == 'skip':
            skipped[test] = list(failures[-5:])
        if action == 'run' and time.monotonic() - last >= 15:
            print(f'SHARD {index} RUN {test} elapsed={time.monotonic()-started:.1f}s', flush=True)
            last = time.monotonic()
        if action == 'output':
            failures.append(event.get('Output', '').rstrip())
            failures = failures[-50:]
        if action == 'fail':
            print(f'SHARD {index} FAIL {test}\n' + '\n'.join(failures), flush=True)
    code = process.wait()
    missing = set(group) - observed
    incomplete = set(group) - passed - set(skipped)
    # Required feature cases may never disappear behind an optional skip.
    required_skips = {name for name in skipped if name.startswith(('TestGraphDraftB', 'TestGraphPerformance', 'TestReviewTokenCache'))}
    print(f'SHARD {index} exit_code={code} elapsed={time.monotonic()-started:.1f}s covered={len(observed)}/{len(group)} passed={len(passed)}', flush=True)
    for name, reason in skipped.items():
        print(json.dumps({'optional_skip': name, 'observed_output': reason}), flush=True)
    if code or missing or incomplete or required_skips:
        print(f'SHARD {index} missing={sorted(missing)} incomplete={sorted(incomplete)} required_skips={sorted(required_skips)}\n' + '\n'.join(failures), flush=True)
        return 1
    return 0


def main():
    plan = json.loads(Path('tests/supervision_go_schedule.json').read_text())
    assert plan['version'] == 1 and 1 <= plan['parallelism'] <= 16
    assert 1 <= plan['groups'] <= 16 and 0 < plan['test_timeout_seconds'] <= 240
    assert all(isinstance(v, (float, int)) and 0 <= v < 10000 for v in plan['observed_seconds'].values())
    inventory = subprocess.run(['go', 'list', '-json', './...'], capture_output=True, text=True, check=True).stdout
    decoder = json.JSONDecoder()
    packages, tested = [], []
    while inventory.strip():
        record, end = decoder.raw_decode(inventory.lstrip())
        inventory = inventory.lstrip()[end:]
        packages.append(record['ImportPath'])
        if record.get('TestGoFiles') or record.get('XTestGoFiles'):
            tested.append(record['ImportPath'])
    engine = 'swarm.local/companion/internal/engine'
    if tested != [engine]:
        raise SystemExit('Test package inventory changed: update shard coverage before running.')
    # Build every package; the disjoint schedule covers the one discovered test package.
    subprocess.run(['go', 'build', './...'], check=True)
    with tempfile.TemporaryDirectory(prefix='swarm-go-suite-') as folder:
        binary = Path(folder)/'companion.test'
        subprocess.run(['go', 'test', '-c', '-o', str(binary), engine], check=True)
        listing = subprocess.run([str(binary), '-test.list', '.'], capture_output=True, text=True, check=True)
        names = [name for name in listing.stdout.splitlines() if re.match(r'^(Test|Example|Fuzz)\w*$', name)]
        groups, isolated = partition(names, plan)
        print(f'GO SUITE package={engine} tests={len(names)} shards={len(groups)} coverage=disjoint-complete compile=once', flush=True)
        codes = [run(groups[i], i, binary, engine, plan['test_timeout_seconds']) for i in range(isolated)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=plan['parallelism']) as pool:
            codes.extend(pool.map(lambda pair: run(pair[1], pair[0]+isolated, binary, engine, plan['test_timeout_seconds']), enumerate(groups[isolated:])))
        if any(codes):
            raise SystemExit(1)
        print(f'PASS full discovered Go suite: {len(names)} tests; all shards exited 0.', flush=True)
        print('Optional Go skips are listed explicitly above, never counted as passed; live-provider probes and opt-in browser recipes are not implied.', flush=True)


if __name__ == '__main__':
    main()
