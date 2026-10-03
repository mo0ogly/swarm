#!/usr/bin/env python3
"""Live acceptance bridge. Requires an external CUA browser driver, never a fixture.

The engine starts a fresh, expiring challenge. The supervisor must perform the
read-only UI recipe through CUA while this process is alive and return its DOM
observations. No archived observation, API shortcut or extra agent is accepted.
This supervised recipe is deliberately not in the unattended unit-test suite.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import time

p = argparse.ArgumentParser()
p.add_argument('--work', required=True)
p.add_argument('--task', required=True)
p.add_argument('--directory', required=True)
a = p.parse_args()
directory = Path(a.directory)
directory.mkdir(parents=True, exist_ok=True)
nonce = secrets.token_hex(16)
started = time.time()
request = directory / 'request.json'
response = directory / ('response-' + nonce + '.json')
challenge = {'nonce': nonce, 'started_unix': started, 'expires_unix': started + 45,
             'work': a.work, 'task': a.task, 'operation': 'CUA Enter -> rendered recovery preview -> Escape -> focus',
             'response': str(response)}
tmp = directory / ('request-' + nonce + '.tmp')
tmp.write_text(json.dumps(challenge), encoding='utf-8')
os.replace(tmp, request)
while not response.exists():
    if time.time() > challenge['expires_unix']:
        raise RuntimeError('Fresh CUA browser response missing: no recorded JSON substituted')
    time.sleep(0.1)
r = json.loads(response.read_text(encoding='utf-8'))
assert r['nonce'] == nonce and r['driver'] == 'CUA actual browser interaction'
assert started <= r['observed_unix'] <= time.time() <= challenge['expires_unix']
assert a.work in r['url'] and '127.0.0.1:18792/' in r['url']
assert r['opened']['dialog_open'] and r['opened']['theme'] in ('etat', 'sombre')
text = r['opened']['text']
assert 'Depuis le refus' in text and 'Consigne de reprise modifiée' in text
assert 'Critères restant à vérifier' in text
# A new refusal can bind the corrected handoff unchanged. The current changed
# proof may be the engine instead; require its rendered stale-proof verdict,
# not a filename that only applied to a previous review.
assert 'Contenu modifié depuis l’avis' in text
assert not r['closed']['dialog_open'] and r['closed']['focus_visible']
assert r['closed']['focused_action'] == 'recovery-preview-' + a.task
assert Path(r['screenshot']).is_file()
record = {'result': 'PASS', 'execution': 'Fresh CUA UI interaction requested during this engine control, not an archived DOM fixture',
          'challenge': challenge, 'observation': r}
(directory / ('receipt-' + nonce + '.json')).write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(record, ensure_ascii=False, indent=2))
