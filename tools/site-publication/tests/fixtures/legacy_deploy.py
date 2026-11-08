"""Activate an exact static Swarm release with retained rollback and witnesses.

Run on the existing VPS with a JSON configuration path argument. Clone the
verified baseline under the deployment lock, add only the authorized bilingual
configured article route and homepage cards, apply the hash-bound delta, then
verify the complete resulting inventory before activation. Configuration
contains public file hashes, archive paths and the previously observed release,
service identities and LIA witnesses, never credentials. Uses the existing
deployment lock; stale baseline, unsafe archive or existing destination refuses.
No Nginx configuration, certificates, DNS, service reload or LIA files change.
"""
from pathlib import Path, PurePosixPath
import fcntl
import hashlib
import json
import os
import subprocess
import sys
import shutil
import tarfile
import urllib.error
import urllib.request


def digest(data):
    """Return SHA256 of the complete supplied public file or response bytes."""
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    """Inventory regular public files and reject symlinks inside a release."""
    assert not any(p.is_symlink() for p in root.rglob('*'))
    return {str(p.relative_to(root)): digest(p.read_bytes()) for p in root.rglob('*') if p.is_file()}


def fetch(url):
    """Return observed status and body, including HTTP error responses.

    Transport/TLS failures propagate; no retries or certificate bypass occur.
    This function has no product timeout or success fallback.
    """
    try:
        with urllib.request.urlopen(url) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def service_identity():
    """Read existing Nginx and LIA backend state without controlling services."""
    return subprocess.check_output(['systemctl', 'show', 'nginx', 'lia-backend', '--property=ActiveState,MainPID,ExecMainStartTimestamp'], text=True)


def check_lia(witnesses):
    """Compare the three established LIA witnesses and health semantics."""
    observed = {}
    for path, expected in witnesses.items():
        status, body = fetch('https://liascan.tech' + path)
        actual = {'status': status, 'sha256': digest(body)}
        assert actual == expected, ('LIA witness changed', path, actual)
        if path == '/api/health':
            assert json.loads(body)['status'] == 'ok'
        observed[path] = actual
    return observed


def main():
    """Validate, activate, verify every HTTPS file and preserve the prior release.

    Activation is an atomic owned symlink replacement. A post-activation failure
    restores the original pointer, verifies its public homepage and LIA witnesses,
    records the original failure and any rollback failure, then propagates it.
    This is content deployment evidence, not engine or global LIA qualification.
    """
    config = json.loads(Path(sys.argv[1]).read_text())
    root = Path('/var/www/swarm-agentic')
    with (root / '.deployment.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        baseline = config['baseline']
        current = root / 'current'
        previous = current.resolve()
        assert str(previous) == baseline['release']
        assert inventory(previous) == baseline['inventory'], 'Baseline file bytes changed'
        assert service_identity() == baseline['services']
        assert {p: digest(Path(p).read_bytes()) for p in baseline['configHashes']} == baseline['configHashes']
        check_lia(baseline['liaWitnesses'])
        archive_path = Path(config['archive'])
        assert digest(archive_path.read_bytes()) == config['archiveSha256']
        slug = config['articleSlug']
        assert slug.startswith('swarm-') and slug.replace('-', '').isalnum()
        prefix = 'articles/' + slug + '/'
        expected = config['inventory']
        destination = root / 'releases' / config['releaseId']
        shutil.copytree(previous, destination)
        assert inventory(destination) == baseline['inventory'], 'Baseline copy changed'
        archive_inventory = config['deltaInventory']
        assert {n for n in expected if expected[n] != baseline['inventory'].get(n)} == set(archive_inventory)
        added = set(expected) - set(baseline['inventory'])
        assert set(baseline['inventory']) <= set(expected), 'No removal authorized'
        assert added and all(n.startswith(prefix) for n in added), 'Only the single authorized article route may be added'
        assert all(n in ('index.html', 'fr.html', 'en.html') or n.startswith(prefix) for n in archive_inventory), 'Unrelated change refused'
        with tarfile.open(archive_path) as archive:
            members = archive.getmembers()
            assert len(members) == len(archive_inventory) == len({m.name for m in members})
            for member in members:
                name = PurePosixPath(member.name)
                assert member.isfile() and not name.is_absolute() and '..' not in name.parts and member.name in archive_inventory
                data = archive.extractfile(member).read()
                assert digest(data) == archive_inventory[member.name]
                target = destination / member.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                target.chmod(0o644)
        assert inventory(destination) == expected
        subprocess.run(['runuser', '-u', 'www-data', '--', 'test', '-r', str(destination / 'fr.html')], check=True)
        evidence = root / 'deployments' / config['releaseId']
        evidence.mkdir(parents=True, exist_ok=False)
        (evidence / 'baseline.json').write_text(json.dumps(baseline, indent=2) + '\n')
        next_link = root / ('current.next.' + config['releaseId'])
        next_link.symlink_to(destination)
        os.replace(next_link, current)
        try:
            for name, expected_hash in expected.items():
                status, data = fetch('https://swarm-agentic.com/' + name)
                assert status == 200 and digest(data) == expected_hash, ('Public file mismatch', name, status)
            lia = check_lia(baseline['liaWitnesses'])
            assert service_identity() == baseline['services']
            assert {p: digest(Path(p).read_bytes()) for p in baseline['configHashes']} == baseline['configHashes']
        except BaseException as error:
            failure = {'originalFailure': repr(error), 'restoredRelease': str(previous)}
            try:
                rollback_link = root / ('current.rollback.' + config['releaseId'])
                rollback_link.symlink_to(previous)
                os.replace(rollback_link, current)
                for name in ('fr.html', 'en.html'):
                    status, data = fetch('https://swarm-agentic.com/' + name)
                    assert status == 200 and digest(data) == baseline['inventory'][name]
                check_lia(baseline['liaWitnesses'])
                failure['rollbackVerified'] = True
            except BaseException as rollback_error:
                failure['rollbackFailure'] = repr(rollback_error)
            (evidence / 'failure.json').write_text(json.dumps(failure, indent=2) + '\n')
            print(json.dumps(failure), flush=True)
            raise
        result = {'state': 'LIVE_HTTPS_VERIFIED', 'release': str(destination), 'previousRelease': str(previous), 'verifiedFiles': len(expected), 'articlesUpdated': len({n.split('/')[1] for n in archive_inventory if n.startswith('articles/')}), 'languagePages': len([n for n in archive_inventory if n.endswith(('/fr.html', '/en.html'))]), 'newArticleRoutes': 1, 'nginxAndLiaConfigsUnchanged': True, 'serviceIdentitiesUnchanged': True, 'liaWitnesses': lia, 'reloads': 0}
        (evidence / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        (evidence / 'public-inventory.json').write_text(json.dumps(expected, indent=2) + '\n')
        temporary = root / ('inventory.' + config['releaseId'] + '.json')
        temporary.write_text(json.dumps(expected, indent=2) + '\n')
        os.replace(temporary, root / 'inventory.json')
        print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
