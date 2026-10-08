#!/usr/bin/env python3
"""Run every default Go test/example/fuzz seed, in disjoint subprocess groups.
The engine observes this command's real exit status, not a previous receipt.
"""
import concurrent.futures, hashlib, os, re, subprocess, sys
listing = subprocess.run(['go', 'test', '-list', '.', './...'], capture_output=True, text=True)
if listing.returncode:
    print(listing.stdout + listing.stderr); sys.exit(listing.returncode)
names = sorted(set(line for line in listing.stdout.splitlines() if re.fullmatch(r'(?:Test|Example|Fuzz)[A-Za-z0-9_]+', line)))
if not names:
    raise SystemExit('No runnable Go tests discovered; refusing empty success')
groups = [[] for _ in range(4)]
for name in names:
    groups[int(hashlib.sha256(name.encode()).hexdigest(), 16) % len(groups)].append(name)
assert sorted(n for group in groups for n in group) == names
print('Full default-suite coverage:', len(names), 'named tests/examples/fuzz targets;', [len(g) for g in groups], 'per group', flush=True)
def run(item):
    i, group = item
    command = ['go', 'test', '-count=1', '-timeout', '250s', '-run', '^(' + '|'.join(group) + ')$', './...']
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    return i, result.returncode, result.stdout
failed = False
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
    futures = [executor.submit(run, item) for item in enumerate(groups) if item[1]]
    for future in concurrent.futures.as_completed(futures):
        i, code, output = future.result()
        print('Group', i+1, 'exit', code, '\n'+output, flush=True)
        failed |= code != 0
sys.exit(1 if failed else 0)
