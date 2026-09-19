"""Hierarchical coordination through the public CLI and deterministic processes.

Scopes own tasks, workers receive local instructions, and the engine relays
reports to their planner. No peer protocol or paid model calls are used.
"""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid
from organized_fixture import enable_organization, add_owned_tasks

binary = str(Path(sys.argv[1]).resolve())
out = Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True)
scenario = sys.argv[3] if len(sys.argv) > 3 else 'nominal'
assert scenario in ('nominal', 'shared', 'restart')
root = Path(tempfile.mkdtemp(prefix='swarm-team-recipe-'))
(root / 'docs').mkdir()

def cli(args, data=None):
    p = subprocess.run([binary, '--root', str(root), '--json', *args] +
                       (['--input', '-'] if data is not None else []),
                       input=json.dumps(data) if data is not None else None,
                       text=True, capture_output=True, timeout=20)
    if p.returncode: raise RuntimeError(p.stdout + p.stderr)
    return json.loads(p.stdout) if p.stdout.strip() else None

cli(['init'])
provider = root / 'worker.py'
provider.write_text(r'''
import json,pathlib,re,sys,time
root=pathlib.Path(sys.argv[1]);prompt=sys.stdin.read()
assert 'Coordination structurée' not in prompt
assert 'Le moteur relaie automatiquement' in prompt
task=re.search(r'Tâche ([a-z0-9]+) :',prompt).group(1)
start=time.time()
if task in ['p1','p2']:
 time.sleep(3)
 text='Production vérifiée pour '+task+'\n'
else:
 text=''.join((root/'docs'/f'{p}.md').read_text() for p in ['p1','p2'])
# Deliver the report in the actual worker directory and publish fixture inputs.
local=pathlib.Path('docs');local.mkdir(exist_ok=True)
(local/f'{task}.md').write_text(text)
(root/'docs'/f'{task}.md').write_text(text)
(root/f'{task}-interval.json').write_text(json.dumps([start,time.time()]))
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'Livrable de recette écrit ; le moteur assure la remise.'}}),flush=True)
''')
(root / '.swarm/providers.json').write_text(json.dumps({'schema_version': 1, 'providers': {
    'fixture-worker': {'command': sys.executable, 'args': [str(provider), str(root)], 'env_allow': []}}}))
w = cli(['work', 'create'], dict(schema_version=1, event_id=uuid.uuid4().hex,
    expected_revision=0, title='Recette équipe hiérarchique', objective='Deux productions puis une synthèse',
    scope='Fournisseurs déterministes, aucun modèle IA', criteria=['p1 exact', 'p2 exact', 'Synthèse exacte'], next='Organiser'))['work']
specs = [('p1', []), ('p2', []), ('synthese', ['p1', 'p2'])]
checks = {}
for i, (tid, deps) in enumerate(specs, 1):
    expected = 'Production vérifiée pour ' + tid + '\n' if tid != 'synthese' else 'Production vérifiée pour p1\nProduction vérifiée pour p2\n'
    checks['req-' + str(i)] = [dict(id='exact', command=['python3', '-c',
        "from pathlib import Path; assert Path('docs/" + tid + ".md').read_text() == " + repr(expected)],
        criteria=[1], justification='Comparer le livrable complet au texte attendu.', timeout_seconds=5)]
w = enable_organization(root, cli, w, checks)
w = add_owned_tasks(cli, w, [dict(id=tid, title=tid, requirements=['req-' + str(i)],
    deliverable='docs/' + tid + '.md', criteria=['Texte attendu exact'], depends=deps,
    next='Produire le rapport local demandé') for i, (tid, deps) in enumerate(specs, 1)])
for tid, deps in specs:
    space = root if scenario == 'shared' else root / tid
    space.mkdir(exist_ok=True)
    cli(['profile', w['id'], tid], dict(provider='fixture-worker', role='worker',
        workspace=str(space), capture_output=True, timeout_seconds=30))
cli(['autonomy', w['id'], 'autonome', '2'])
cli(['mission', 'start', w['id']], dict(provider='fixture-worker', role='worker',
    workspace=str(root), capture_output=True, timeout_seconds=30))
log = (out / 'server.log').open('w')
server = subprocess.Popen([binary, '--root', str(root), 'web'], stdout=log, stderr=log)
restarted = False
try:
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        current = cli(['work', 'show', w['id']])['work']
        if scenario == 'restart' and not restarted and sum(t['status']=='running' for t in current['tasks']) == 2:
            server.kill(); server.wait(timeout=5)
            server = subprocess.Popen([binary, '--root', str(root), 'web'], stdout=log, stderr=log)
            restarted = True
        if all(t['status']=='accepted' for t in current['tasks']) and current['planning']['scopes'][0]['state']=='closed': break
        time.sleep(.3)
    else: raise AssertionError([(t['id'],t['status'],t.get('blocker')) for t in current['tasks']])
    agents = cli(['agent', 'list', w['id']])['agents']
    assert len(agents) == 3 and all(x['agent']['status']=='completed' for x in agents)
    intervals = {tid: json.loads((root/(tid+'-interval.json')).read_text()) for tid, _ in specs}
    overlap = max(intervals['p1'][0], intervals['p2'][0]) < min(intervals['p1'][1], intervals['p2'][1])
    assert overlap == (scenario != 'shared')
    assert intervals['synthese'][0] >= max(intervals['p1'][1], intervals['p2'][1])
    handoffs = [e for e in current['planning']['inbox'] if e['kind']=='handoff']
    assert len(handoffs) == 3 and all(e.get('attempt') and e.get('artifacts') for e in handoffs)
    assert all(t['independent_review']['state']=='passed' for t in current['tasks'])
    assert all(t['automatic_validation']['controller'] != t['automatic_validation']['producer_agent_id'] for t in current['tasks'])
    assert scenario != 'restart' or restarted
    result = dict(status='PASS', scenario=scenario, provider='deterministic, no AI calls',
                  workers=3, handoffs=3, independent_reviews=3, planner_closed=True,
                  parallel_producers=overlap, restart=restarted, interventions_after_launch=0)
    (out/'recipe.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result))
finally:
    cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=10);log.close()
    for entry in cli(['agent','list',w['id']])['agents']:
        if entry['agent']['status'] in ('running','starting','queued','stopping'):
            cli(['agent','stop',entry['agent']['id']])
