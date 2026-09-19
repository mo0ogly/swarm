"""Native installer and real Compose lifecycle, isolated project, no model calls.

Run: TMPDIR=/dev/shm python3 tests/install_smoke.py
Requires Go, Docker Engine on this host, Compose v2. Builds the Docker image.
The installer writes deploy/install.env; refuse if an existing configuration exists.
"""
import http.cookiejar
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import urllib.request
import uuid

repo = Path(__file__).resolve().parents[1]
config = repo / 'deploy/install.env'
assert not config.exists(), 'Existing install.env: use a clean checkout for this test'
# Bind mounts must use a directory visible at the same path to the local daemon.
temp = Path(tempfile.mkdtemp(prefix='swarm-install-', dir=Path.home() / '.cache'))
project = temp / 'project with spaces'
agent_home = temp / 'agent home'
project.mkdir()
agent_home.mkdir()
env = dict(os.environ, COMPOSE_PROJECT_NAME='swarm-install-test-' + uuid.uuid4().hex[:10])
compose = ['docker', 'compose', '--env-file', str(config), '-f', str(repo / 'compose.yaml')]

def run(args, **kwargs):
    p = subprocess.run(args, cwd=repo, env=env, text=True, capture_output=True, **kwargs)
    if p.returncode:
        raise RuntimeError(re.sub(r'session-[a-z0-9]+', 'session-REDACTED', p.stdout + p.stderr))
    return p.stdout

try:
    bad = subprocess.run(['./install.sh', '--port', '0'], cwd=repo, capture_output=True)
    assert bad.returncode == 2 and not config.exists()
    native = temp / 'native bin'
    run(['./install.sh', '--mode', 'native', '--bin-dir', str(native), '--project', str(project)])
    initial = json.loads(run([str(native / 'swarm'), '--root', str(project), '--json', 'work', 'list']))
    assert initial is not None
    # Preserve sentinel proves initialization does not remove project content.
    (project / 'keep.txt').write_text('preserve me')
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    run(['./install.sh', '--project', str(project), '--agent-home', str(agent_home), '--port', str(port)], timeout=900)
    assert config.stat().st_mode & 0o777 == 0o600
    logs = run(compose + ['logs', '--no-color', 'swarm'])
    url = re.search(r'http://127\.0\.0\.1:\d+/session/[a-z0-9-]+', logs)[0]
    browser = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    with browser.open(url) as response:
        assert response.status == 200 and b'SWARM' in response.read().upper()
    prefix = compose + ['exec', '-T', 'swarm', 'swarm', '--root', '/workspace', '--json']
    work = json.loads(run(prefix + ['work', 'create', '--input', '-'], input=json.dumps({
        'schema_version': 1, 'event_id': str(uuid.uuid4()), 'expected_revision': 0,
        'title': 'Docker persistence test', 'objective': 'Preserve this mission across recreation',
        'scope': 'Test fixture', 'criteria': ['Mission remains available'], 'next': 'Read the mission'
    })))['work']
    # Same installation refuses to replace a running service.
    refused = subprocess.run(['./install.sh', '--project', str(project), '--agent-home', str(agent_home), '--port', str(port)], cwd=repo, env=env, text=True, capture_output=True)
    assert refused.returncode == 2 and 'fonctionne déjà' in refused.stderr
    run(compose + ['exec', '-T', 'swarm', 'node', '--version'])
    run(compose + ['exec', '-T', 'swarm', 'python3', '--version'])
    run(compose + ['exec', '-T', 'swarm', 'sh', '-c', 'printf persisted > /home/swarm/keep-agent.txt'])
    run(compose + ['down'])
    run(compose + ['up', '-d', '--wait', '--wait-timeout', '60'])
    restored = json.loads(run(prefix + ['work', 'show', work['id']]))
    assert work['id'] in json.dumps(restored)
    assert (agent_home / 'keep-agent.txt').read_text() == 'persisted'
    assert (project / 'keep.txt').read_text() == 'preserve me'
    assert (project / '.swarm/state.db').stat().st_uid == os.getuid()
    print('PASS: native install, Docker build/start, browser session, CLI, running-service guard, persisted mission and agent home, file ownership')
finally:
    if config.exists():
        subprocess.run(compose + ['down'], cwd=repo, env=env, capture_output=True)
        config.unlink()
    shutil.rmtree(temp)
