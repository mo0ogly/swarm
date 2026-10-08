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
COMMON = [
    '00-swarm-besoin.png', '01-swarm-mission.png', '02-swarm-lancement.png',
    '04-swarm-activite.png', '05-casa-catalogue.png', '07-casa-erreur-minimum.png',
    '08-casa-panier.png', '09-casa-confirmation.png', '11-casa-restaurant.png',
    '13-casa-livree.png', '16-casa-mobile-confirmation.png',
    '17-swarm-validation.png', '26-casa-accueil.png',
]


def build(lang, guide, start, output):
    files = [guide, start, 'atelier_swarm.py', 'creer_plan_pedagogique.py']
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
