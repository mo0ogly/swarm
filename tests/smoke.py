"""Fresh-process resume, export/import and CLI presentation acceptance test."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

binary = os.environ['SWARM_BINARY']
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / 'project'; root.mkdir()
    other = Path(td) / 'transported'; other.mkdir()
    def call(*args, data=None, target=root, expected=0):
        command = [binary, '--root', str(target), '--json', *args]
        if data is not None: command += ['--input', '-']
        p = subprocess.run(command, input=None if data is None else json.dumps(data), text=True, capture_output=True)
        assert p.returncode == expected, (command, p.stdout, p.stderr)
        return json.loads(p.stdout)
    call('init')
    request = dict(schema_version=1, event_id='create-smoke', expected_revision=0,
                   title='Reprise multi-session', objective='Retrouver le travail', scope='local',
                   criteria=['Le checkpoint est conservé'], next='Créer la tâche')
    w = call('work', 'create', data=request)['work']
    assert call('work', 'create', data=request)['work']['revision'] == 1
    w = call('task', 'add', w['id'], data=dict(schema_version=1,event_id='task-smoke',expected_revision=1,
                  id='task',title='Vérifier la reprise',deliverable='Rapport de reprise',criteria=['Contexte retrouvé'],owner='session'))['work']
    w = call('task', 'update',w['id'],data=dict(schema_version=1,event_id='start-smoke',expected_revision=2,id='task',status='running'))['work']
    w = call('checkpoint',w['id'],data=dict(schema_version=1,event_id='checkpoint-smoke',expected_revision=3,summary='Tâche commencée, aucun résultat validé',next='Vérifier le processus précédent'))['work']
    resumed = call('resume',w['id'])
    assert resumed['work']['revision'] == 4
    assert 'activité non confirmée' in resumed['resume_markdown']
    assert 'Vérifier le processus précédent' in resumed['resume_markdown']
    assert all(len(line)<=80 for line in resumed['resume_markdown'].splitlines())
    archive = str(Path(td)/'work.zip')
    call('export',w['id'],'--output',archive)
    call('init',target=other)
    imported=call('import','--input',archive,target=other)
    assert imported['work']==resumed['work']
    assert imported['events']==resumed['events']
    print('PASS: fresh processes, duplicate create, active-task warning, 80 columns, portable import')
