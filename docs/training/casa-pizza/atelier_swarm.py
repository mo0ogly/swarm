"""Create a fresh Swarm learning project; never copy runtime state or credentials."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description='Créer un atelier Casa Pizza dans Swarm')
    parser.add_argument('--swarm', required=True, help='Chemin du binaire Swarm installé')
    parser.add_argument('--projet', required=True, help='Nouveau dossier de travail')
    parser.add_argument('--agent', default='codex', help='Programme agent installé et authentifié')
    args = parser.parse_args()
    if args.agent != 'codex':
        parser.error('Cet atelier préconfigure Codex. Configurer un autre agent depuis IA et connexions.')
    binary = Path(args.swarm).expanduser().resolve()
    root = Path(args.projet).expanduser().resolve()
    source = Path(__file__).resolve().parent / 'projet-pizza'
    if not binary.is_file():
        parser.error('Binaire Swarm absent ; compiler ou installer Swarm auparavant.')
    if root.exists() and any(root.iterdir()):
        parser.error('Choisir un dossier neuf ou vide pour préserver le travail existant.')
    agent = shutil.which(args.agent)
    if not agent:
        parser.error('Programme agent absent du PATH ; installer et authentifier cet agent auparavant.')
    root.mkdir(parents=True, exist_ok=True)
    (root / 'docs').mkdir()
    for name in ['AGENTS.md', 'docs/BESOIN.md', '.gitignore']:
        shutil.copy2(source / name, root / name)
    subprocess.run(['git', 'init', '-b', 'codex/formation-pizza', str(root)], check=True, stdout=subprocess.DEVNULL)
    subprocess.run(['git', '-C', str(root), 'add', 'AGENTS.md', 'docs/BESOIN.md', '.gitignore'], check=True)
    subprocess.run(['git', '-C', str(root), '-c', 'user.name=Formation Swarm', '-c', 'user.email=formation@example.invalid',
                    'commit', '-m', 'Définir le besoin de Casa Pizza'], check=True, stdout=subprocess.DEVNULL)

    def cli(parts, value=None):
        cmd = [str(binary), '--root', str(root), '--json', *parts]
        if value:
            cmd += ['--input', '-']
        result = subprocess.run(cmd, input=json.dumps(value) if value else None,
                                text=True, capture_output=True, check=True)
        return json.loads(result.stdout) if result.stdout.lstrip().startswith('{') else {}

    cli(['init'])
    # The installer does not install or authenticate the external agent.
    config = {'schema_version': 1, 'providers': {'codex': {
        'command': agent, 'args': ['exec', '--json', '--sandbox', 'workspace-write', '-'], 'env_allow': []}}}
    (root / '.swarm' / 'providers.json').write_text(json.dumps(config, indent=2))
    work = cli(['work', 'create'], {
        'schema_version': 1, 'event_id': 'atelier-pizza-create', 'expected_revision': 0,
        'title': 'Formation Casa Pizza', 'objective': 'Créer et vérifier une webapp locale de livraison de pizzas',
        'scope': 'Catalogue, panier, commande persistante, suivi et espace restaurant ; démonstration locale',
        'criteria': ['Une commande complète fonctionne', 'Les montants et accès sont protégés', 'Les tests passent'],
        'next': 'Relire docs/BESOIN.md puis lancer la tâche'})['work']
    work = cli(['task', 'add', work['id']], {
        'schema_version': 1, 'event_id': 'atelier-pizza-task', 'expected_revision': work['revision'],
        'id': 'pizza-app', 'title': 'Construire la webapp et ses tests',
        'deliverable': 'Application locale, tests, README et docs/HANDOFF.md',
        'criteria': ['Catalogue, panier et confirmation utilisables', 'Commande persistante et montants validés côté serveur',
                     'Suivi client et accès restaurant protégés', 'Tests métier et README de lancement',
                     'Direction graphique relue sur les écrans desktop et mobile'],
        'next': 'Réalise docs/BESOIN.md et respecte AGENTS.md. Livre code, tests, README et docs/HANDOFF.md. '
                'Python standard library, SQLite et ressources locales. Aucun autre agent et aucune publication. '
                'Ne lance pas de serveur durable. Vérifie montants, erreurs, accès et persistance. '
                'Travaille la direction visuelle décrite dans le besoin et signale les contrôles graphiques non réalisés. '
                'Ne modifie pas .swarm.'})['work']
    print('Projet prêt :', root)
    print('Mission :', work['id'])
    print('Aucun agent lancé et aucune clé copiée. Ouvrir le cockpit sur ce projet puis lancer la tâche après lecture.')


if __name__ == '__main__':
    main()
