"""Public-CLI product-view fixture. Empty isolated root only; no AI or dispatch.

Usage: python3 tests/product_recipe.py BINARY EMPTY_PROJECT
Prints fixture IDs, never a browser session credential.
"""
import json
from pathlib import Path
import subprocess
import sys
import uuid

binary, project = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
project.mkdir(parents=True, exist_ok=True)
assert not any(project.iterdir()), 'Use a new empty project, never a live mission'


def cli(args, data=None):
    result = subprocess.run([str(binary), '--root', str(project), '--json'] + args,
                            input=None if data is None else json.dumps(data),
                            text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr + result.stdout)
    return json.loads(result.stdout)


def request(action, preparation=None, **fields):
    payload = dict(version=1, event_id=uuid.uuid4().hex, action=action,
                   expected_revision=preparation['revision'] if preparation else 0,
                   **fields)
    if preparation:
        payload['preparation_id'] = preparation['id']
    return cli(['prepare', action] + ([preparation['id']] if preparation else []) +
               ['--input', '-'], payload)


cli(['init'])
methods = cli(['prepare', 'methods'])
assert all(m['available'] for m in methods)
assert any(m['id'] == 'ks-product' for m in methods)
p = request('create', title='Casa Pizza — product navigation recipe',
            method='ks-product', text='Explicit UI fixture. No AI call or agent execution.')
p = request('save', p, document='brief',
            text='Isolated product-view acceptance recipe; no product result accepted.')
p = request('adopt-brief', p, sha256=p['documents']['brief']['sha256'])
ids = ['ARCH', 'DESIGN', 'CATALOGUE', 'API', 'ORDER', 'HISTORY']
titles = ['Architecture et contrats', 'Design system', 'Catalogue des pizzas',
          'API et persistance', 'Valider la commande', 'Consulter les commandes']
deps = [[], ['ARCH'], ['DESIGN'], ['ARCH'], ['CATALOGUE', 'API'], ['API']]
tasks = []
for index, identity in enumerate(ids):
    tasks.append(dict(id=identity, phase=['architecture', 'design-system'][index]
                      if index < 2 else 'implement', title=titles[index], role='worker',
                      scope='Isolated recipe and demonstration files',
                      deliverable='docs/' + identity.lower() + '.md', depends=deps[index],
                      criteria=['Observable behaviour verified for ' + titles[index]],
                      proof='Reproducible check and candidate evidence',
                      entry='Accepted prerequisites with fresh evidence',
                      validation='Verify criteria and failures', delivery='Fresh independent review',
                      stop='Stop on invalid scope or reached limit', max_attempts=1, max_tool_calls=10))
stories = [
    dict(id='s01-catalogue', title='Choisir une pizza', user='Client', value='Composer son panier',
         criteria=['Les articles choisis apparaissent dans le panier'], complexity=2,
         depends=[], task_ids=['CATALOGUE']),
    dict(id='s02-order', title='Commander une pizza', user='Client', value='Recevoir une confirmation',
         criteria=['Commande persistée et confirmation affichée'], complexity=3,
         depends=['s01-catalogue'], task_ids=['CATALOGUE', 'API', 'ORDER']),
    dict(id='s03-history', title='Retrouver mes commandes', user='Client', value='Consulter mes achats',
         criteria=['Seules mes commandes sont visibles'], complexity=2,
         depends=['s02-order'], task_ids=['API', 'HISTORY']),
]
journeys = [dict(id='buy', title='Acheter une pizza', goal='Du catalogue à la confirmation',
                story_ids=['s01-catalogue', 's02-order']),
            dict(id='account', title='Suivre mes commandes', goal='Retrouver mes achats',
                 story_ids=['s03-history'])]
for index in range(4, 27):
    identity = f's{index:02}-later'
    stories.append(dict(id=identity, title=f'Fonction future {index:02}', user='Client',
                        value='Prochain incrément', criteria=['Résultat observable à réaliser'],
                        complexity=1, depends=[], task_ids=[]))
    journeys.append(dict(id=f'later-{index:02}', title=f'Parcours futur {index:02}',
                         goal='Recette de pagination ; non planifié', story_ids=[identity]))
plan = dict(version=1, objective='Product navigation acceptance recipe',
            assumptions=['Explicit fixture, no autonomous implementation'], questions=[], tasks=tasks,
            product=dict(mode='greenfield', journeys=journeys, stories=stories))
raw = json.dumps(plan, ensure_ascii=False, separators=(',', ':'))
assert len(raw.encode()) < 16000
p = request('save', p, document='plan', text=raw)
p = request('validate-plan', p, sha256=p['documents']['plan']['sha256'])
review = cli(['prepare', 'conversion', p['id']])
p = request('create-missions', p, sha256=p['documents']['plan']['sha256'],
            expected_work_revision=review['work_revision'])
assert not (project / '.claude').exists()
work = cli(['work', 'show', p['work_id']])['work']
assert work['plans'][-1]['spec']['product'] == plan['product']
assert len(work['tasks']) == 6 and all(t['launch_held'] for t in work['tasks'])
assert cli(['agent', 'list', p['work_id']])['agents'] == []
print(json.dumps(dict(project=str(project), preparation=p['id'], work=p['work_id'],
                      journeys=len(journeys), stories=len(stories), bytes=len(raw.encode())), indent=2))
