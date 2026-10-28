from pathlib import Path
import datetime, hashlib, json, subprocess, sys, tarfile

B = Path(__file__).resolve().parent
ROOT = Path.cwd()

def inputs():
    paths = [ROOT / 'go.mod', ROOT / 'go.sum']
    paths += list((ROOT / 'internal').rglob('*.go')) + list((ROOT / 'cmd').rglob('*.go'))
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}

def run(name, command):
    if (B / (name + '-run.json')).exists():
        raise SystemExit('Preserve existing attempt ' + name)
    before = inputs()
    (B / (name + '-inputs.json')).write_text(json.dumps(before, indent=2) + '\n')
    with tarfile.open(B / (name + '-launch-sources.tar.gz'), 'w:gz') as archive:
        for path in sorted((ROOT / 'internal/engine').glob('agent_identity_config_launch_linux*.go')):
            archive.add(path, arcname=str(path.relative_to(ROOT)))
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with (B / (name + '.jsonl')).open('wb') as out, (B / (name + '.stderr')).open('wb') as err:
        result = subprocess.run(command, stdout=out, stderr=err)
    after = inputs()
    changed = sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p))
    events = []
    for line in (B / (name + '.jsonl')).read_text().splitlines():
        try:
            events.append(json.loads(line))
        except ValueError:
            pass
    names = {action: sorted({e['Test'] for e in events if e.get('Action') == action and e.get('Test')}) for action in ['pass', 'fail', 'skip']}
    record = dict(command=command, exit=result.returncode, start=start,
                  end=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  counts={k: len(v) for k, v in names.items()}, names=names,
                  changed_inputs=changed, input_count=len(before), overlay=None)
    (B / (name + '-run.json')).write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps(dict(step=name, exit=result.returncode, counts=record['counts'], changed_inputs=changed)), flush=True)
    if changed or result.returncode:
        raise SystemExit(result.returncode or 1)
    if command[:2] == ['go', 'test']:
        assert names['pass'] and not names['fail'] and not names['skip'], record
    return record

if __name__ == '__main__':
    label, scope = sys.argv[1:3]
    regex = '^TestAgentIdentityConfigLaunch' if scope == 'launch' else '^TestAgentIdentity(ConfigLaunch|ConfigClient|ConfigService|Channel)'
    for mode in ['normal', 'race']:
        run(mode + '-' + scope + '-' + label, ['go', 'test', *(['-race'] if mode == 'race' else []), '-json', './internal/engine', '-run', regex, '-count=1', '-timeout=0'])
    if scope != 'launch':
        run('vet-' + label, ['go', 'vet', './internal/engine'])
        run('build-' + label, ['go', 'build', '-o', '/tmp/swarm-identity-config-launch-03c-fixture-' + label, './cmd/swarm'])
