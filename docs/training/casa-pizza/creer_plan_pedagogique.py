"""Create an unexecuted demonstration plan through the public Swarm CLI."""
import argparse
import json
from pathlib import Path
import subprocess
import uuid


def main():
    parser = argparse.ArgumentParser(description='Create the six-task Casa Pizza teaching plan')
    parser.add_argument('--swarm', required=True, help='Installed Swarm executable')
    parser.add_argument('--projet', required=True, help='Existing initialized workshop project')
    args = parser.parse_args()
    binary = Path(args.swarm).expanduser().resolve()
    root = Path(args.projet).expanduser().resolve()
    if not binary.is_file() or not (root / '.swarm').is_dir():
        parser.error('Use an installed executable and an initialized workshop project.')

    def cli(parts, data):
        result = subprocess.run([str(binary), '--root', str(root), '--json', *parts, '--input', '-'],
                                input=json.dumps(data), capture_output=True, text=True)
        if result.returncode:
            raise SystemExit(result.stderr or result.stdout)
        return json.loads(result.stdout)['work']

    def mutate(parts, work, **fields):
        return cli(parts, dict(schema_version=1, event_id=uuid.uuid4().hex,
                               expected_revision=work.get('revision', 0), **fields))

    work = mutate(['work', 'create'], {}, title='Casa Pizza — Plan pédagogique',
                  objective='Apprendre le graphe et les contrôles sans lancer des agents',
                  scope='Démonstration de configuration ; tâches non exécutées',
                  criteria=['Lire les dépendances', 'Distinguer contrôles et décision humaine'],
                  next='Examiner le graphe')
    specs = [
        ('brief', '01 Clarifier le besoin', [], ['Périmètre et critères relus par une personne'], 'docs/BESOIN.md'),
        ('server', '02 API et persistance', ['brief'], ['Calcul serveur, accès et persistance testés'], 'app.py'),
        ('client', '03 Catalogue et panier', ['brief'], ['Catalogue, filtre, quantités et erreurs utilisables'], 'static/index.html'),
        ('journey', '04 Commande et suivi', ['server', 'client'], ['Commande et statuts vérifiés avec HTTP'], 'docs/recette/navigateur.json'),
        ('verify', '05 Tests automatiques', ['journey'], ['Les onze tests du candidat réussissent'], 'docs/FORMATEUR.md'),
        ('human', '06 Recette humaine', ['verify'], ['Parcours navigateur examiné avant acceptation'], 'docs/FORMATEUR.md'),
    ]
    for task_id, title, dependencies, criteria, deliverable in specs:
        work = mutate(['task', 'add', work['id']], work, id=task_id, title=title,
                      depends=dependencies, criteria=criteria, deliverable=deliverable,
                      next='Démonstration pédagogique : ne pas lancer de producteur. Examiner critères et dépendances.')
    print('Teaching plan:', work['id'])
    print('Six tasks created. No agent launched, policy authorized, or existing task changed.')
    print('Run once per workshop; running again creates another demonstration plan.')


if __name__ == '__main__':
    main()
