"""Build the two public training kits from an explicit file allowlist."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

ROOT = Path(__file__).resolve().parent
PROJECT = [
    'app.py', 'README.md', 'AGENTS.md', '.gitignore',
    'tests/__init__.py', 'tests/test_app.py', 'tests/test_http.py',
    'static/index.html', 'static/app.js', 'static/style.css',
    'static/restaurant.html', 'static/restaurant.js',
    'docs/BESOIN.md', 'docs/HANDOFF.md', 'docs/FORMATEUR.md',
    'docs/REFONTE.md', 'docs/pizza-app.md',
]
PROJECT += [f'static/images/{name}.svg' for name in
            ['margherita', 'reine', 'fromages', 'vegetarienne', 'diavola', 'calzone']]
TRAINING_FRAMES = ['00-besoin-en.png','00-besoin-fr.png','01-catalogue.png','02-filtre.png','03-minimum.png','04-quantite.png','05-confirmee.png','06-connexion.png','07-refus-acces.png','08-commande-recue.png','09-preparation.png','10-suivi-preparation.png','11-livraison.png','12-livree.png','13-graphe-en.png','13-graphe-fr.png','14-tache-en.png','14-tache-fr.png','15-humaine-en.png','15-humaine-fr.png','16-controle-en.png','16-controle-fr.png','17-apercu-en.png','17-apercu-fr.png','18-graphe-sombre.png','19-commande-en.png','19-commande-fr.png']
COMMON = [
    '00-swarm-besoin.png', '01-swarm-mission.png', '02-swarm-lancement.png',
    '04-swarm-activite.png', '05-casa-catalogue.png', '07-casa-erreur-minimum.png',
    '08-casa-panier.png', '09-casa-confirmation.png', '11-casa-restaurant.png',
    '13-casa-livree.png', '16-casa-mobile-confirmation.png',
    '17-swarm-validation.png', '26-casa-accueil.png',
]


def build(lang, guide, start, output):
    files = ['Formation_Swarm_Casa_Pizza.pdf', 'Swarm_Casa_Pizza_Training_EN.pdf', 'Formation_Swarm_Casa_Pizza.docx', 'Swarm_Casa_Pizza_Training_EN.docx', start, 'atelier_swarm.py', 'creer_plan_pedagogique.py']
    files += ['tutoriels/swarm-logo.png', 'tutoriels/index.html', 'tutoriels/player.css', 'tutoriels/player.js', 'tutoriels/storyboard.json', 'tutoriels/manifest.json', 'tutoriels/README.md']
    files += [f'tutoriels/frames/{name}' for name in TRAINING_FRAMES]
    files += [f'tutoriels/media/{module}-{language}{suffix}' for module in ['01-plan', '02-controles', '03-client', '04-restaurant'] for language in ['fr', 'en'] for suffix in ['.mp4', '.gif', '.txt', '.vtt', '-poster.jpg']]
    files += ['tutoriels/media/graph-overview-fr.mp4', 'tutoriels/media/graph-overview-fr-poster.png', 'tutoriels/media/graph-overview-fr.vtt']
    files += [f'projet-pizza/{name}' for name in PROJECT]
    images = COMMON + [f'{name}-{lang}.jpg' for name in [
        '18-graph', '19-task', '20-validation-human', '21-validation-command',
        '23-validation-effect', '24-validation-auto', '25-graph-edit']]
    images += [f'guide-{name}-{lang}.png' for name in ['graph', 'validation']]
    files += [f'captures/{name}' for name in images]
    assert len(files) == len(set(files))
    for name in files:
        if not (ROOT / name).is_file():
            raise FileNotFoundError(name)
    with ZipFile(ROOT / output, 'w', compression=ZIP_DEFLATED) as archive:
        for name in sorted(files):
            archive.write(ROOT / name, name)
    with ZipFile(ROOT / output) as archive:
        assert archive.testzip() is None
        assert set(archive.namelist()) == set(files)
    print(f'{output}: {len(files)} files')


if __name__ == '__main__':
    build('fr', 'Formation_Swarm_Casa_Pizza.docx', 'COMMENCER_ICI.txt',
          'Kit_Formation_Swarm_Casa_Pizza.zip')
    build('en', 'Swarm_Casa_Pizza_Training_EN.docx', 'START_HERE_EN.txt',
          'Swarm_Casa_Pizza_Training_Kit_EN.zip')
