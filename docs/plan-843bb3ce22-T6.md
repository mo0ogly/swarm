# Task result — plan-843bb3ce22-T6

## Outcome in two sentences

QW1 (REQ-VER-01/02) et QW2 (REQ-LINK-01) sont vérifiés fonctionnels : version
serveur affichée avec état explicite et comparaison sans réseau imposé ;
lien vers une mission supprimée affiche un message clair avec CTA « Choisir
une mission ». Le code applicatif préexistait dans la copie ; cette tentative
a comblé l'absence totale de test (Go et navigateur) et produit des preuves
réelles datées (build, tests, recette navigateur FR/EN x 2 thèmes, captures).

## Identity and scope

- Work / task / attempt / producer : w-843bb3ce22cff2965c5e77b6 /
  plan-843bb3ce22-T6 / a-b5276a88ccd9e4fc7645061d / agent
  7610d86e-6538-440e-a4a4-51b4767defc8 ; départ 3/3 ; révision du travail 157.
- Role and assigned scope : worker — REQ-VER-01/02 et REQ-LINK-01
  uniquement (version/build, comparaison sans réseau imposé, message et
  choix de mission après lien supprimé). Pas d'Administration, pas de CI.
- Base / candidate revision and dirty changes : branche
  `codex/engine-review-contract`, HEAD `097e745a9b1404f16ed6fdf4ad32124a3f661e05`,
  arbre de travail avec de nombreuses modifications préexistantes non liées
  à cette tâche (autres lots de la même mission, présents dès le début de
  cette tentative — préservés sans y toucher).
- State : implémentation QW1/QW2 vérifiée fonctionnelle avec preuves
  d'exécution réelles. Ceci n'est pas une acceptation moteur ni une revue
  indépendante.

## Findings the responsible planner must know

- Le code de QW1 et QW2 était déjà présent dans la copie au démarrage de
  cette tentative (`runtime_health.go`, `web/cockpit.js`,
  `web/runtime-health.js`, `web/index.html`, i18n FR/EN), vraisemblablement
  produit par la tentative précédente interrompue
  (`6e065899-4cd0-4b4a-9239-fb7098a7644d`, attempt `a-65dc0a043d9d19c694a76548`).
  Rien de tout cela n'était testé : aucun test Go n'exerçait
  `serverVersion()`, aucun test navigateur ne couvrait ni le badge de
  version ni le cas précis d'un lien direct vers une mission supprimée
  (seule la suppression via la corbeille était couverte, par
  `tests/q9_lifecycle_ui.cjs`, hors périmètre T6).
- Cette tentative a donc uniquement ajouté des tests et exécuté des preuves,
  sans modifier le comportement applicatif de QW1/QW2 (aucun écart constaté
  entre le code existant et les critères REQ-VER-01/02 / REQ-LINK-01).
- `run_limits_admin.go`, `run_limits_web.go`, `run_limits_cli.go`,
  `admin.js`/`admin` view dans `web/index.html`, etc. sont présents dans la
  copie mais appartiennent à d'autres lots (Administration, T2/T3) — non
  touchés, non vérifiés par cette tentative.
- La recette navigateur ajoutée (`tests/quick_wins_ui.cjs`,
  `npm run test:quickwins`) n'est pas câblée dans `.github/workflows/ci.yml` :
  cela relève de REQ-CI-01 / QW3 (Lot 5), une tâche distincte non confiée ici.

## Changes and verification

| Requirement | Change / file | Exact check and environment | Observed effect | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| REQ-VER-01/02 (QW1) | Vérification de `runtime_health.go:32-68`, `web/runtime-health.js:9-19`, `web/index.html` (badge `#server-version`), i18n FR/EN | `go build ./...` puis `go test . -run TestServerVersionReflectsBuildAndLocalGitWithoutNetwork -v` (ajouté par cette tentative) | Build sans erreur ; test vérifie `Source`==HEAD git réel, cohérence `Compared`/`Current`, dégradation explicite sans dépôt git, sans exécuter la moindre requête réseau | PASS | sortie `go test` (13 tests du fichier passent), voir commande ci-dessous |
| REQ-VER-01/02 (QW1) | idem, bout en bout | `npm run test:quickwins` (Puppeteer + Chrome réel, racine isolée avec un dépôt git vide) | Badge affiche « Version : 097e745a9b14\* · différente de la copie locale » (FR) / « Version: 097e745a9b14\* · different from the local checkout » (EN) dans les 2 thèmes ; 0 requête hors de l'hôte local | PASS | `test-results/quick-wins/result.json`, `version-{fr,en}-{etat,sombre}.png` |
| REQ-LINK-01 (QW2) | Vérification de `web/cockpit.js:11` (`noticeMissionSupprimee`) et `web/cockpit.js:200` (détection au bootstrap) | `npm run test:quickwins`, navigation vers `?work=w-does-not-exist-<lang>` avec une mission réelle existante | Vue bascule sur « Gérer les missions », message exact affiché, un seul bouton CTA « Choisir une mission »/« Choose a mission », clic → focus clavier sur `#manage-search` | PASS | `test-results/quick-wins/result.json`, `deleted-mission-{fr,en}-{etat,sombre}.png` |
| Non-régression i18n | `locales/en.json`, `web/i18n-en.js` | `npm run i18n:build` puis `git diff --stat` | Diff identique avant/après (aucune dérive entre la source FR et la sortie EN générée) | PASS | commande ci-dessous |

Aucun diagnostic invalide ni échec/récupération distinct requis pour ces
deux critères : QW1/QW2 n'ont pas de chemin d'erreur serveur (le badge se
dégrade sans lever d'erreur, le lien supprimé ne déclenche pas d'appel API
en échec — il est détecté côté client avant tout appel inutile).

### Commandes exactes et statuts de sortie

```
$ go build ./...
(exit 0, aucune sortie)

$ go test . -run TestServerVersionReflectsBuildAndLocalGitWithoutNetwork -v
Go test: 13 passed in 1 packages   (exit 0)

$ npm run i18n:build && git diff --stat -- locales/en.json web/i18n-en.js
locales/en.json | 97 ++++...
web/i18n-en.js  | 97 ++++...
2 files changed, 192 insertions(+), 2 deletions(-)   (exit 0 ; diff identique au diff préexistant, aucune dérive)

$ make build
CGO_ENABLED=0 go build -trimpath -o bin/swarm .   (exit 0)

$ npm run test:quickwins
fr: version badge shows a non-empty, explicit state (...)
fr: version badge captured in both themes without a network request beyond the local server
fr: deleted-mission link shows a clear notice (...)
fr: "Choisir une mission" CTA focuses the mission search field
en: version badge shows a non-empty, explicit state (...)
en: version badge captured in both themes without a network request beyond the local server
en: deleted-mission link shows a clear notice (...)
en: "Choisir une mission" CTA focuses the mission search field
no request left the local server for version display or the deleted-mission notice (REQ-VER-02: no imposed network access)
(exit 0)
```

## APEX / PDCA checkpoint

- Analysis / PLAN : lecture du diff existant (`git diff`) sur les fichiers
  cités par la mémoire de reprise ; confirmation que QW1/QW2 sont déjà codés
  mais non testés.
- Execution / DO : ajout d'un test Go ciblé (`runtime_health_test.go`) et
  d'une recette navigateur dédiée (`tests/quick_wins_ui.cjs` +
  `package.json` → `test:quickwins`). Aucune modification du comportement
  applicatif existant.
- Verification / CHECK : `go build`, test Go ciblé, `make build`, recette
  navigateur complète FR/EN x 2 thèmes avec captures, vérification manuelle
  du contenu des captures (absence de secret/lien de session).
- Adjustment / ACT : aucun écart trouvé entre le code existant et les
  critères QW1/QW2 ; aucune correction applicative nécessaire.
- Recovery limits : mode observation autorisé pour cette mission (pas de
  plafond moteur d'appels/tentatives) ; compteurs mesurés mais non bloquants
  selon la consigne utilisateur en vigueur pour cette tentative.

## Next action and limits

Les deux critères QW1 et QW2 assignés à T6 sont démontrés par une recette
navigateur réelle et un test Go réel, avec preuves horodatées dans
`docs/T6-quick-wins.md` et `test-results/quick-wins/`. Cette tâche s'arrête
ici conformément à la consigne (« Finir dès que les deux critères sont
démontrés »).

Reste hors de ce périmètre, pour le responsable de plan : câblage CI de
`npm run test:quickwins` (REQ-CI-01/QW3, Lot 5), et revue indépendante de T6
avant le Lot 6 documentation (gate de livraison du contrat de tâche —
non réalisée ici, cette tentative ne modifie pas la base Swarm et ne
s'auto-déclare pas acceptée).

## Complément de supervision — contrôles moteur, 30 septembre

La première revue a classé les critères inconnus car `engine_controls` était
vide. Aucun nouveau développement n'est nécessaire pour ce constat : les
preuves doivent être exécutées et transmises par le moteur. La politique
préautorise entry, version (test Go), browser (build courant et recette réelle),
validation (empreintes et captures), delivery (fraîcheur et rapports). Le
script échoue au premier sous-processus ou contrôle en erreur, utilise un
binaire temporaire et une base de recette isolée. Aucun état réel de mission
n'est modifié par le script. La sortie moteur fait autorité sur l'exécution,
le code ci-dessous en précise la portée.

```python
#!/usr/bin/env python3
"""Execute T6 checks on a temporary binary and an isolated browser fixture."""
import hashlib
import pathlib
import subprocess
import tempfile
import sys
import json

ROOT = pathlib.Path(__file__).resolve().parents[2]
for name in ('runtime_health.go', 'runtime_health_test.go', 'web/runtime-health.js',
             'web/cockpit.js', 'tests/quick_wins_ui.cjs'):
    print('SOURCE', name, hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), flush=True)

def run(command):
    print('COMMAND', command, flush=True)
    subprocess.run(command, cwd=ROOT, check=True, timeout=150)

mode = sys.argv[1] if len(sys.argv) > 1 else 'all'
files = ('runtime_health.go', 'runtime_health_test.go', 'web/runtime-health.js', 'web/cockpit.js', 'tests/quick_wins_ui.cjs')
fingerprints = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in files}
out = ROOT / 'test-results/t6-engine'
if mode in ('entry', 'all'):
    for name in files:
        assert (ROOT / name).is_file(), name
    assert (ROOT / 'node_modules/puppeteer').is_dir()
    print('PASS prerequisites: source, tests and browser dependency present')
if mode in ('version', 'all'):
    run(['go', 'test', '.', '-run', '^TestServerVersionReflectsBuildAndLocalGitWithoutNetwork$', '-count=1', '-v'])
if mode in ('browser', 'all'):
    with tempfile.TemporaryDirectory(prefix='swarm-t6-control-') as directory:
        binary = str(pathlib.Path(directory) / 'swarm')
        run(['go', 'build', '-o', binary, '.'])
        run(['node', 'tests/quick_wins_ui.cjs', binary, str(out)])
    (out / 'sources.json').write_text(json.dumps(fingerprints, sort_keys=True))
if mode in ('validation', 'delivery', 'all'):
    assert json.loads((out / 'sources.json').read_text()) == fingerprints, 'stale browser evidence'
    result = json.loads((out / 'result.json').read_text())
    assert result['status'] == 'PASS' and not result['errors'] and not result['offHost']
    for lang in ('fr', 'en'):
        for theme in ('etat', 'sombre'):
            for kind in ('version', 'deleted-mission'):
                assert (out / f'{kind}-{lang}-{theme}.png').stat().st_size > 1000
    print('PASS current browser artifacts, no JS errors, no external request, eight screenshots')
if mode in ('delivery', 'all'):
    for name in ('docs/T6-quick-wins.md', 'docs/plan-843bb3ce22-T6.md'):
        report = (ROOT / name).read_text()
        assert 'REQ-VER-01' in report and 'REQ-LINK-01' in report
    print('PASS required reports cover both requirement IDs; independent review remains separate')
assert mode in ('entry', 'version', 'browser', 'validation', 'delivery', 'all')
print('PASS T6 control', mode, flush=True)

```
