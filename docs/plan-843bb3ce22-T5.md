# T5 — Recette navigateur FR/EN x thèmes en CI (REQ-CI-01)

Tâche `plan-843bb3ce22-T5`, work `w-843bb3ce22cff2965c5e77b6`, tentative
courante `a-07eec605cc6254e0ea672554` (départ 2/2), agent
`auto-359f3f40958ed97c4804`. Reprise d'un rapport antérieur (tentative
`a-ece2de601dad5897265633ab`) : ce rapport le remplace en totalité (un seul
rapport relayé) et couvre le candidat réellement courant.

## 0. Écart constaté par rapport au rapport repris

Le rapport antérieur documentait un run GitHub Actions réel (36628140996,
succès) mais sur le commit `c5f1dea91310608f1b4b327a0cf6349e1bbc2c55`,
branche `codex/browser-ci-proof` — **pas** le HEAD courant de cette
tentative. Vérifié :

- `git merge-base --is-ancestor c5f1dea HEAD` → **non ancêtre** : `c5f1dea`
  n'est pas dans l'historique de la branche courante `codex/engine-review-contract`.
- `git diff c5f1dea HEAD -- .github/workflows/ci.yml tests/i18n_ui.cjs` →
  **aucune différence** sur ces deux fichiers (le branchement CI et le
  script de recette sont octet pour octet identiques entre les deux
  révisions).
- `git diff c5f1dea HEAD -- web/i18n-en.js` → diffère (traductions ajoutées
  depuis, ex. rubrique Administration limites agents), mais la clé
  `"Ajouter une IA": "Add an AI connection"` utilisée par la recette est
  inchangée (`web/i18n-en.js:94`).
- Aucun run GitHub Actions n'existait pour le HEAD courant
  `7110a0b805caf4be1bb09a57b776176b8dc52a68` avant cette tentative
  (`gh run list --branch codex/engine-review-contract` : dernier run
  `pull_request` sur `092ab9b`, run `push`/`workflow_dispatch` absent pour
  `7110a0b`). Conformément au contrat (« changed code invalidates old
  evidence »), l'ancien run distant n'est donc pas retenu comme preuve du
  candidat courant ; il reste cité comme preuve historique de branchement,
  pas comme preuve d'exécution sur ce HEAD.

Correctif apporté cette tentative : reproduction complète du cycle
vert/rouge bloquant/vert **sur le HEAD réellement courant**, et
déclenchement d'un run GitHub Actions réel lié à ce HEAD (§3).

## 1. État du dépôt à l'entrée

```
pwd: /home/fpizzi/workspace/swarm-engine-contract/source
git rev-parse --show-toplevel: (identique)
git status --short: (vide, arbre propre)
HEAD: 7110a0b805caf4be1bb09a57b776176b8dc52a68 (branche codex/engine-review-contract)
upstream github/codex/engine-review-contract: identique à HEAD (déjà poussé)
```

## 2. Constat repris de T1 (inchangé, revérifié)

`tests/i18n_ui.cjs` (`npm run test:i18n-ui`) reste la recette Puppeteer
déjà existante et réutilisée telle quelle : parcourt cockpit + connexion IA
+ `prepare.html` en FR puis EN, bascule les deux thèmes (`etat`/`sombre`) à
chaque étape, capture 4 combinaisons langue×thème par écran, vérifie
l'absence d'erreur JS (`assert.deepEqual(errors, [])`) et l'intégrité des
données de mission au changement de langue. Chrome système présent
(`/usr/bin/google-chrome`), `PUPPETEER_SKIP_DOWNLOAD: 'true'` déjà défini au
niveau du job CI.

Branchement CI présent dans le HEAD courant (`.github/workflows/ci.yml`,
job `checks`, après `make smoke`, aucun `continue-on-error` — échec
bloquant par construction) :

```
      - run: npm run test:i18n-ui
        env:
          CHROME_BIN: /usr/bin/google-chrome
```

Aucune seconde source de vérité : `npm run test:i18n-ui` reste l'unique
point d'entrée.

## 3. Preuves — cycle vert / rouge bloquant / vert, HEAD 7110a0b

Racine de travail unique. Binaire reconstruit à chaque étape
(`CGO_ENABLED=0 go build -trimpath -o bin/swarm .`, assets web embarqués —
`//go:embed web/*`). Logs horodatés dans `docs/ci-evidence/` (non suivi par
git, `.gitignore:13`).

### 3.1 Vert — baseline, HEAD courant, avant toute mutation

`docs/ci-evidence/T5-head7110a0b-baseline-green-20260930T131040Z.log` :
```
> test:i18n-ui
> node tests/i18n_ui.cjs bin/swarm test-results/i18n
PASS bilingual web UI
```
Sortie script : `PASS bilingual web UI`, aucune exception non interceptée.

### 3.2 Rouge bloquant — régression injectée sur HEAD courant

Régression : `web/i18n-en.js:94`,
`"Ajouter une IA": "Add an AI connection"` →
`"Ajouter une IA": "REGRESSION-T5-INJECTED"`. Rebuild (asset embarqué),
`npm run test:i18n-ui` —
`docs/ci-evidence/T5-head7110a0b-regression-red-20260930T131040Z.log` :
```
AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:
+ actual - expected
+ 'REGRESSION-T5-INJECTED'
- 'Add an AI connection'
    at .../tests/i18n_ui.cjs:...
  actual: 'REGRESSION-T5-INJECTED',
  expected: 'Add an AI connection',
```
Code de sortie process : **1** (constaté directement, sans filtrage :
`npm run test:i18n-ui; echo TEST_EXIT:$?` → `TEST_EXIT:1`). Une étape
`- run: npm run test:i18n-ui` sans `continue-on-error` échoue le job GitHub
Actions correspondant — bloquant par construction du workflow.

### 3.3 Vert — régression retirée, HEAD restauré

Traduction restaurée à l'identique. Rebuild,
`npm run test:i18n-ui` —
`docs/ci-evidence/T5-head7110a0b-restored-green-20260930T131040Z.log` :
```
> test:i18n-ui
> node tests/i18n_ui.cjs bin/swarm test-results/i18n
PASS bilingual web UI
```
Code de sortie : **0** (constaté directement : `TEST_EXIT:0`).
`git status --short` : vide. `rg -n "REGRESSION-T5-INJECTED" web/i18n-en.js` :
aucune correspondance — aucune trace résiduelle, arbre de travail identique
à l'entrée.

## 4. Preuve distante liée au HEAD courant

Run GitHub Actions déclenché explicitement sur ce HEAD (`workflow_dispatch`,
le workflow l'autorise — `.github/workflows/ci.yml:5`) :

```
gh workflow run CI --repo mo0ogly/swarm --ref codex/engine-review-contract
→ run 36719933441, event workflow_dispatch, headSha 7110a0b805caf4be1bb09a57b776176b8dc52a68
```

Statut au moment de la rédaction : **en cours** (`in_progress`), suivi en
tâche de fond bornée (poll `gh run view --json status` jusqu'à
`completed`, sans sommeil bloquant en avant-plan). Résultat final à
reporter dès disponible ; si ce rapport est relayé avant complétion, ce
run reste vérifiable publiquement :
https://github.com/mo0ogly/swarm/actions/runs/36719933441

Preuve historique conservée pour mémoire (branchement identique, HEAD
différent) : run `36628140996`, commit `c5f1dea9131...`, conclusion
`success`, étape `Run npm run test:i18n-ui` → `success`, log runner
`PASS bilingual web UI`. Ne qualifie pas `7110a0b` à elle seule ; combinée à
l'identité de fichiers §0 et à la reproduction locale §3, elle établit que
le même branchement produit le même résultat sur le HEAD courant.

## 5. Couverture FR/EN x thèmes clair/sombre

Confirmée par lecture directe de `tests/i18n_ui.cjs` (fichier identique à
la version revue en T1, voir §0) : cockpit (vue graphe, modale connexion
IA) et `prepare.html` capturés dans les 4 combinaisons langue×thème
(FR/clair, FR/sombre, EN/clair, EN/sombre), avec assertions de contenu
traduit à chaque bascule de langue (pas seulement des captures), et garde
de navigation testée (changement de langue bloqué si brouillon non vide,
message traduit vérifié).

## 6. Résultat des trois critères — HEAD 7110a0b

| Critère | État | Preuve |
| --- | --- | --- |
| CI verte sur parcours couverts existants | **Confirmé localement** ; run distant lié à ce HEAD en cours (§4) | §3.1, §3.3 ; run 36719933441 |
| Échec bloquant sur régression injectée puis retirée | **PASS** | §3.2 (exit 1, AssertionError), §3.3 (exit 0 après restauration, arbre propre) |
| FR/EN x thèmes clair/sombre couverts | **PASS** | §5 |

## 7. Limites et prochaine action

- Le run distant `36719933441` était en cours au moment de la rédaction ;
  son verdict final n'est pas encore inclus dans ce texte. Vérifier
  `gh run view 36719933441 --repo mo0ogly/swarm --json status,conclusion`
  avant d'accepter la tâche si ce champ n'a pas été complété ci-dessus.
- Preuve de blocage distant (run GitHub échouant réellement sur la
  régression) non rejouée à distance cette tentative, pour ne pas laisser
  un commit cassé visible sur la branche partagée sans autorisation
  explicite de push d'un état rouge ; le blocage est démontré localement
  (§3.2) avec le même binaire/scénario que celui exécuté en CI.
- Revue indépendante et gate fraîche avant QW3 (T6) restent à obtenir côté
  moteur ; ce rapport ne constitue pas cette revue.


## Complément du superviseur — 30 septembre, après 15 h

La CI distante 36719933441 est terminée avec succès sur
`7110a0b805caf4be1bb09a57b776176b8dc52a68` : jobs checks et install réussis,
étape `Run npm run test:i18n-ui` réussie. Le statut « en cours » ci-dessus
est l'observation historique du worker, désormais remplacée par ce constat.

Le refus local provenait du contrôleur `tools/verification/t5_ci.py` resté
lié au commit `c5f1dea9131…` : son contrôle d'identité refusait package.json.
Le contrôleur référence maintenant le commit et le run actuels. Son empreinte
change ; la politique de validation doit être réautorisée dans Swarm avant
rejeu. Aucun contrôle n'est supprimé et aucune acceptation n'est forcée.

Limite : les changements locaux du catalogue de modèles Claude 5.5 sont
postérieurs à ce commit ; cette CI distante ne démontre pas ces changements.
Les contrôles navigateur et mutation sont rejoués sur le candidat local.
