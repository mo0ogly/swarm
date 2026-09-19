"""Organisation fictive réservée au générateur de captures, jamais au serveur réel.

Le générateur crée d'abord un projet temporaire sans fournisseur. Nous ajoutons
un scénario de présentation en pause. Aucun processus, résultat ou avis IA n'est
simulé comme ayant été exécuté : toutes les tâches restent à faire.
"""
import json
import pathlib
import sqlite3
import sys

root = pathlib.Path(sys.argv[1])
assert root.name.startswith('swarm-readme-'), 'Projet temporaire de captures requis'
with sqlite3.connect(root / '.swarm/state.db') as db:
    wid, raw = db.execute('SELECT id, body FROM works LIMIT 1').fetchone()
    work = json.loads(raw)
    work['title'] = 'Démo simulée — Équipe de recherche accessible'
    work['scope'] = 'Organisation fictive en pause. Aucun appel IA et aucun résultat revendiqué.'
    work['planning'] = {
        'version': 1, 'paused': True, 'provider': 'IA de planification · démo',
        'max_tasks': 8, 'max_decisions': 12, 'max_activations': 20,
        'activations': 0, 'decisions': 0, 'inbox': [], 'reviewer_required': True,
        'reviewer': {'provider': 'IA de revue · démo', 'provider_digest': '',
                     'max_calls': 10, 'calls': 0, 'authorized': ''},
        'scopes': [
            {'id': 'root', 'objective': work['objective'], 'requirements': ['req-1', 'req-2'],
             'revision': 1, 'generation': 0, 'state': 'ready', 'activations': 0},
            {'id': 'accessibilite', 'parent': 'root', 'objective': 'Vérifier le parcours clavier',
             'requirements': ['req-2'], 'revision': 1, 'generation': 0, 'state': 'ready', 'activations': 0}
        ]
    }
    for task in work['tasks']:
        task['plan_role'] = 'worker'
        task['scope_id'] = 'accessibilite' if task['id'] == 'verification' else 'root'
        task['requirements'] = ['req-2'] if task['id'] == 'verification' else ['req-1']
        task['launch_held'] = True
        task['next'] = task['deliverable']
    db.execute('UPDATE works SET body=? WHERE id=?', (json.dumps(work), wid))
