# T3 — recette du superviseur sur le candidat courant

## Origine et attribution

Travail réel `w-115c11e8f802a4f98c3def32`, dépôt swarm-action-skills,
base Git `1d9570bd4ef617130c6be96b7ec88844fdbcd00e` avec modifications non commitées.
Le premier exécutant a commencé les compteurs puis a été interrompu après
25 outils (`a-cacc533574c699dddab19104`). Le superviseur a complété le même
périmètre : interface, traductions, tests et correction du retour de focus
`web/cockpit.js` constatée dans la modale réelle. Le second exécutant ne doit
pas s'attribuer ces modifications ou les captures. Les limites restent intactes.

## Contrat de mesure

- Une ligne par agent et tentative, même pour une tâche ensuite acceptée.
- État du processus distinct de la validation enregistrée de la tâche, qui
  couvre toutes ses tentatives et ne garantit pas la fraîcheur des preuves.
- `metrics_version=1` : Read/Glob/Grep = lectures ;
  Write/Edit/MultiEdit/file_change = écritures ; autres outils = non classés.
- Erreurs cumulées persistantes après succès. Répétitions nom+entrée sous
  nouveaux IDs ; retransmission d'un même ID dédoublonnée.
- Perte de visibilité ou outil en attente : mesure partielle, observations
  conservées ; terminal et ancien format : catégories inconnues, pas zéro.
- Les tests ne sont pas déduits des commandes mixtes. Leur nombre reste inconnu.
- Un départ n'est pas un appel LLM. Appels internes du fournisseur inconnus.
  Coûts et jetons seulement si rapportés, pas d'estimation présentée comme réel.

## Contrôles exécutés par le superviseur

| Contrôle | Résultat | Limite |
|---|---|---|
| `go test ./... -run 'TestAttemptMetrics|TestAttemptLedger|TestLoopGuard' -count=1 -timeout 180s` | PASS, 0,271 s | événements structurés contrôlés, pas autonomie LLM |
| `go test -race ./... -run 'TestAttemptMetrics|TestAttemptLedger|TestLoopGuard' -count=1 -timeout 180s` | PASS, 2,168 s | mesures/gardes ciblées |
| `npm test` | PASS | contrats DOM doubles + catalogue FR/EN, navigateur ci-dessous séparé |
| `python3 tools/agent-workflows/check.py` | PASS, 9 méthodes | configuration statique |
| `git diff --check` | PASS | espaces et marqueurs de conflits |
| `go vet ./...` | PASS sur complément moteur | analyse statique |

La suite Go complète est encore en cours ; aucune affirmation de succès global.
La suite démarrée avant les derniers ajustements de libellés ne suffit pas à
prouver un candidat final complet sans les contrôles ciblés courants.

## Recette CLI réelle

`bin/swarm --lang fr mission spending w-115c11e8f802a4f98c3def32`
et l'équivalent `--lang en`, plus `--json mission spending` : sortie réelle
du moteur et historique persistant, sans base utilisée comme fixture ni mutation.

Cas interrompu réel T2 `a-1838917a21fb55cd54481c1c` : 25 outils observés,
catégories historiques inconnues, coût/usage absents.
Cas terminé réel T2 `a-4fea4cb374a0cf60b5cc2680` : 12 outils observés,
20/16528 jetons rapportés, coût fournisseur environ 0,65 USD, catégories
historiques inconnues. La tâche porte une acceptation enregistrée mais sa preuve
est devenue périmée par le complément T3 : ne pas confondre ces deux faits.

## Recette navigateur réelle

Serveur construit depuis le candidat, port 18792 ; affichage
`devel · 1d9570bd4ef6*`, provenance injectée explicite, état dirty.
Bouton « Où vont les appels et les coûts ? » dans la Conduite, ouverture par
Entrée, lecture des deux mêmes tentatives ; Échap ferme la modale et restaure
le focus au bouton même après remplacement par actualisation.

Captures `docs/screenshots/clear-launch-recovery/t3-ledger-{fr,en}-{light,dark}.png`
prises par le superviseur via navigateur réel. La mission et ses rapports restent
en français car ce sont des contenus utilisateur ; les libellés du bilan doivent
être anglais dans la variante en. Les quatre variantes ont été rendues et examinées ; ouverture par Entrée,
fermeture Échap et retour du focus au bouton confirmés dans chacune.
Aucune erreur ou alerte console dans les relevés CUA. Le superviseur réalise
ces opérations, pas l'exécutant.

## Sources exactes

`agents_store.go`, `activity_description.go`, `loop_guard.go`,
`mission_insights.go`, `attempt_ledger_test.go`, `web/mission-insights.js`,
`web/cockpit.js` (restauration de focus seulement),
`tests/attempt_ledger_test.cjs`, `locales/en.json`, `web/i18n-en.js`,
`package.json`. Aucun statut, budget ou preuve historique supprimé.
