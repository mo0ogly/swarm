from pathlib import Path
import datetime, hashlib, json, subprocess, sys, tarfile

B = Path(__file__).resolve().parent
ROOT = Path.cwd()

def inputs():
    metadata = json.loads((B/'root-package.json').read_text())
    observed = json.loads(subprocess.check_output(['go','list','-json','.'],text=True))
    assert observed.get('EmbedFiles') == metadata.get('EmbedFiles'), 'embedded file set changed'
    paths = [ROOT/'go.mod',ROOT/'go.sum']
    paths += list((ROOT/'internal').rglob('*.go'))+list((ROOT/'cmd').rglob('*.go'))+list(ROOT.glob('*.go'))
    paths += [ROOT/name for name in metadata['EmbedFiles']]
    overlay = json.loads((B/'overlay.json').read_text())['Replace']
    return {str(p.relative_to(ROOT)):hashlib.sha256(Path(overlay.get(str(p.resolve()),p)).read_bytes()).hexdigest() for p in sorted(set(paths))}

def run(name, command):
    if (B / (name + '-run.json')).exists():
        raise SystemExit('Preserve existing attempt ' + name)
    if command[0]=='go': command=command[:2]+['-overlay',str((B/'overlay.json').resolve())]+command[2:]
    before = inputs()
    (B / (name + '-inputs.json')).write_text(json.dumps(before, indent=2) + '\n')
    with tarfile.open(B / (name + '-launch-sources.tar.gz'), 'w:gz') as archive:
        for path_name in ['internal/enrollmentconfig/store_linux.go','internal/enrollmentconfig/path_linux.go',
                     'internal/enrollmentbootstrap/bootstrap_linux.go','internal/enrollmentbootstrap/bootstrap_linux_test.go',
                     'internal/engine/main.go','internal/engine/agent_identity_bootstrap_linux.go',
                     'internal/engine/agent_identity_bootstrap_linux_test.go']:
            archive.add(ROOT / path_name, arcname=path_name)
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
                  changed_inputs=changed, input_count=len(before), overlay=str((B/'overlay.json').resolve()))
    (B / (name + '-run.json')).write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps(dict(step=name, exit=result.returncode, counts=record['counts'], changed_inputs=changed)), flush=True)
    if changed or result.returncode:
        raise SystemExit(result.returncode or 1)
    if command[:2] == ['go', 'test']:
        assert names['pass'] and not names['fail'] and not names['skip'], record
    return record

if __name__ == '__main__':
    label = sys.argv[1]
    for mode in ['normal','race']:
        flags = ['-race'] if mode == 'race' else []
        run(mode+'-packages-'+label,['go','test',*flags,'-json','./internal/enrollmentbootstrap','./internal/enrollmentconfig','-count=1','-timeout=0'])
        run(mode+'-engine-'+label,['go','test',*flags,'-json','./internal/engine','-run','^TestAgentIdentity(Bootstrap|ConfigLaunch|ConfigClient|ConfigService|Channel)','-count=1','-timeout=0'])
    run('vet-'+label,['go','vet','./internal/enrollmentbootstrap','./internal/enrollmentconfig','./internal/engine'])
    run('build-'+label,['go','build','-o','/tmp/swarm-identity-bootstrap-03d-fixture-'+label,'./cmd/swarm'])
