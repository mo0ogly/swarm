# T6 — Version du serveur et liens vers missions supprimées

Tâche `plan-843bb3ce22-T6`, périmètre : REQ-VER-01/02 (QW1) et REQ-LINK-01 (QW2)
uniquement. Révision de base : `097e745a9b1404f16ed6fdf4ad32124a3f661e05`
(branche `codex/engine-review-contract`), arbre de travail avec modifications
préexistantes non liées à cette tâche (autres lots en cours dans la même copie).

Le code applicatif de QW1 et QW2 existait déjà dans l'arbre de travail au
démarrage de cette tentative (probablement produit par une tentative
interrompue précédente sur la même copie). Le travail de cette tentative a
consisté à : vérifier ce code par lecture, combler l'absence totale de test
(aucun test ne couvrait ni `serverVersion()` ni le lien vers une mission
supprimée), l'exécuter réellement et consigner des preuves datées.

## QW1 — REQ-VER-01/02 : version serveur, comparaison sans réseau imposé

### Implémentation (préexistante, vérifiée)

- `runtime_health.go:32-68` : `ServerVersion` ajouté à `RuntimeHealth`.
  `serverVersion()` lit la révision VCS embarquée par le compilateur Go
  (`debug.ReadBuildInfo`, disponible depuis Go 1.18 sans action de build
  supplémentaire) et la compare au HEAD git local (`gitState(s.root)`,
  déjà utilisé ailleurs dans le moteur pour détecter une dérive de
  checkpoint — aucune requête réseau, uniquement `git -C <root> rev-parse
  HEAD`). Si la révision de build est absente → `Available=false` (état
  « inconnu » explicite, pas de valeur inventée). Si le dépôt local est
  absent ou n'a pas de HEAD → `Compared=false` (comparaison dégradée,
  explicite, pas une supposition).
- `web/runtime-health.js:9-19` (`renderVersion`) : affiche le badge
  `#server-version` avec 3 états distincts — inconnu, connu sans
  comparaison possible, connu et comparé (à jour / différent) — chacun
  avec un `title` explicatif.
- `web/index.html` : badge `#server-version` ajouté dans le rail latéral,
  visible sur toutes les vues.
- Chaînes FR/EN : `locales/en.json` / `web/i18n-en.js` (regénérés via
  `npm run i18n:build`, aucune dérive constatée — diff identique avant/après).

### Lacune comblée par cette tentative

Aucun test, Go ou navigateur, n'exerçait `serverVersion()` ni le badge
`#server-version` avant cette tentative. Ajouté :

- `runtime_health_test.go` — `TestServerVersionReflectsBuildAndLocalGitWithoutNetwork` :
  vérifie que `Source` correspond au HEAD git réel du dépôt, que
  `Compared`/`Current` sont cohérents quand `Available=true`, et que sur une
  racine sans dépôt git (`t.TempDir()`) la comparaison est explicitement
  dégradée (`Compared=false`, `Current=false`, `Source=""`) sans erreur.
- `tests/quick_wins_ui.cjs` — recette navigateur réelle (Puppeteer, Chrome
  local, aucun mock du binaire) : ouvre le cockpit sur une racine isolée
  (nouveau dépôt git vide, donc HEAD différent du binaire testé — exerce la
  branche « comparaison fonctionnelle mais différente », pas seulement le
  cas totalement dégradé), vérifie que le badge affiche un texte non vide et
  explicite en FR et EN, capture les 2 thèmes, et **trace toutes les requêtes
  réseau de la page pour prouver qu'aucune ne sort du serveur local**
  (`offHost` vide dans `test-results/quick-wins/result.json`).

### Preuves QW1

| Preuve | Commande | Résultat |
| --- | --- | --- |
| Test Go ciblé | `go test . -run TestServerVersionReflectsBuildAndLocalGitWithoutNetwork -v` | PASS |
| Build complet | `go build ./...` | Sans erreur |
| Recette navigateur | `npm run test:quickwins` (→ `node tests/quick_wins_ui.cjs bin/swarm test-results/quick-wins`) | PASS, voir `test-results/quick-wins/result.json` |
| Capture FR clair | `test-results/quick-wins/version-fr-etat.png` | Badge : « Version : 097e745a9b14* · différente de la copie locale » |
| Capture FR sombre | `test-results/quick-wins/version-fr-sombre.png` | idem, thème sombre |
| Capture EN clair | `test-results/quick-wins/version-en-etat.png` | « Version: 097e745a9b14* · different from the local checkout » |
| Capture EN sombre | `test-results/quick-wins/version-en-sombre.png` | idem, thème sombre |
| Absence de requête réseau externe | assertion `offHost` dans `result.json` | `[]` — aucune requête hors de l'hôte local du serveur Swarm |

Le `*` après le hash signale `vcs.modified=true` (arbre de travail non
propre au moment du build), confirmé indépendamment par
`go version -m bin/swarm | grep vcs`.

## QW2 — REQ-LINK-01 : lien vers une mission supprimée

### Implémentation (préexistante, vérifiée)

- `web/cockpit.js:11` (`noticeMissionSupprimee`) : message clair + bouton
  « Choisir une mission » qui bascule sur la vue `manage` et place le focus
  sur le champ de recherche des missions.
- `web/cockpit.js:200` (bootstrap) : au chargement, si `?work=<id>` est
  fourni et que cet identifiant n'existe pas parmi les missions retournées
  par `/api/v1/works` (et qu'au moins une mission existe), la vue bascule
  sur `manage` au lieu de tenter de charger un travail inexistant, et
  `noticeMissionSupprimee()` est appelée. Le cas « aucune mission du tout »
  garde son message existant distinct (pas de confusion entre les deux
  situations).
- Chaînes FR/EN déjà présentes (même build i18n que QW1).

### Lacune comblée par cette tentative

Aucun test ne couvrait le cas précis d'un lien direct vers une mission
supprimée (seule la suppression elle-même, via la corbeille, était couverte
par un test existant — `tests/q9_lifecycle_ui.cjs`, hors périmètre de cette
tâche). Ajouté dans `tests/quick_wins_ui.cjs` : navigation directe vers
`?work=w-does-not-exist-<lang>` avec au moins une mission réelle existante,
vérification du message exact, présence d'un unique bouton CTA, et que le
clic sur ce bouton ramène bien le focus clavier sur le champ de recherche
(`#manage-search`) tout en gardant la vue « Gérer les missions » ouverte.

### Preuves QW2

| Preuve | Commande | Résultat |
| --- | --- | --- |
| Recette navigateur | `npm run test:quickwins` | PASS |
| Capture FR clair | `test-results/quick-wins/deleted-mission-fr-etat.png` | Message + CTA visibles |
| Capture FR sombre | `test-results/quick-wins/deleted-mission-fr-sombre.png` | idem |
| Capture EN clair | `test-results/quick-wins/deleted-mission-en-etat.png` | idem |
| Capture EN sombre | `test-results/quick-wins/deleted-mission-en-sombre.png` | « The mission opened by this link no longer exists: it has been deleted or purged. » + « Choose a mission » |
| Focus clavier après clic CTA | assertion Puppeteer `document.activeElement.id==='manage-search'` | Vérifié dans les 2 langues |

## Vérification manuelle du contenu (gate de validation)

Les 8 captures d'écran et `test-results/quick-wins/result.json` ont été
relues manuellement : aucun secret (clé API, jeton, mot de passe) ni lien de
session (`/session/<token>`, cookie, URL signée) n'apparaît. Le seul
identifiant visible est l'identifiant de mission `w-f07c7274a8f90f22698058b9`
dans la liste « Gérer les missions », qui est un identifiant fonctionnel
public de l'interface, pas un secret. Cette tâche ne touche pas l'écran de
diagnostic (« Copier le diagnostic », QW5, hors périmètre T6) : aucune
capture ne provient de cet écran.

## Ce qui n'a pas été refait

- Aucune modification du moteur de configuration/Administration
  (`run_limits_admin.go`, `admin.js`, etc.) : hors périmètre REQ-VER/REQ-LINK
  de cette tâche, présents dans la copie pour d'autres lots.
- Aucune modification de `.github/workflows/ci.yml` : l'intégration en CI de
  cette recette est REQ-CI-01 / QW3, une tâche distincte (Lot 5). La recette
  `tests/quick_wins_ui.cjs` est prête à y être branchée (`npm run
  test:quickwins`) mais son câblage CI n'est pas dans le périmètre confié ici.
- Aucun nouvel inventaire global : réutilisation du diff et des rapports
  déjà présents dans la copie, conformément à la consigne de reprise.

## Limites et risques résiduels

- Le test Go `TestServerVersionReflectsBuildAndLocalGitWithoutNetwork`
  passe un `t.Skip` si aucun `git` n'est disponible dans l'environnement
  d'exécution (CI sans `git` sur le PATH) ; dans cet environnement, `git`
  était disponible et le test s'est exécuté normalement (voir preuve ci-dessus).
- La comparaison de version dépend de la présence d'un checkout git au
  chemin `--root` du serveur en production ; en installation Docker sans
  `.git` monté, l'état restera explicitement « comparaison indisponible »
  plutôt qu'une fausse confirmation — comportement conforme à REQ-VER-02
  mais à garder en tête pour la documentation d'installation (hors périmètre
  ici).
