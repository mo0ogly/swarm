"""Current public planning contracts for deterministic process recipes.

A fixture executable named claude exercises the tool-free adapter protocol. It
never invokes Claude or a paid model. All state changes use the public CLI.
"""
import json
import sys
import uuid


def planning(cli, work, action, **fields):
    value = cli(['planning', action, work['id']], dict(
        schema_version=1, event_id=uuid.uuid4().hex,
        expected_revision=work['revision'], **fields))
    return value.get('work', value)


def enable_organization(root, cli, work, checks):
    executable = root / 'claude'
    executable.write_text('#!' + sys.executable + '\n' + r'''
import json,sys
prompt=sys.stdin.read()
context=None
for i,c in enumerate(prompt):
 if c=='{':
  try:context=json.loads(prompt[i:]);break
  except json.JSONDecodeError:pass
assert isinstance(context,dict)
if 'report' in context:
 report=context['report'].strip()
 response={'reason':'Revue déterministe du texte fourni ; les contrôles moteur restent nécessaires.','criteria':[{'index':i+1,'verdict':'pass' if report else 'unknown','evidence':report if report else 'Rapport vide'} for i in range(len(context['criteria']))]}
else:
 tasks=context.get('tasks',[])
 response={'input_events':[e['id'] for e in context['events']], 'reason':'Retours lus par le planificateur de recette.', 'operations':[]}
 if tasks and all(t['status']=='accepted' for t in tasks):response['operations']=[{'kind':'close'}]
print(json.dumps({'type':'result','result':json.dumps(response)}),flush=True)
''')
    executable.chmod(0o700)
    config_path = root / '.swarm/providers.json'
    config = json.loads(config_path.read_text())
    config['providers']['fixture-planner'] = {'command': str(executable), 'args': [], 'env_allow': []}
    config_path.write_text(json.dumps(config))
    return planning(cli, work, 'enable', provider='fixture-planner',
                    max_tasks=10, max_decisions=30, max_activations=40,
                    checks=checks)



def add_owned_tasks(cli, work, tasks):
    scope = work['planning']['scopes'][0]
    work = planning(cli, work, 'claim', scope='root', scope_revision=scope['revision'],
                    holder='fixture-setup', lease_seconds=60)
    scope = work['planning']['scopes'][0]
    return planning(cli, work, 'decide', scope='root', scope_revision=scope['revision'],
                    holder=scope['holder'], generation=scope['generation'],
                    input_events=[e['id'] for e in work['planning']['inbox'] if e['scope']=='root' and not e.get('decision')],
                    reason='Affecter la tâche au responsable racine avec son critère explicite.',
                    operations=[dict(kind='task', **task) for task in tasks])
