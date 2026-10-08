# T3 — État actuel et historique : correction et preuves

Mission `w-01567e073c1ed2f3d4c71c9e`, tâche `plan-01567e073c-T3`.
Producteur historique : `1426ec9c-5a9d-4d45-85ec-316341a236eb`, tentative
`a-008c49f126275727492dfa40`, terminée normalement à 11 appels sur 12.
Complément d'implémentation et recette : **superviseur Codex**, après arrêt du
producteur. Aucune troisième tentative, aucun remboursement de quota.
HEAD `cc3069dc7bb61b90168d21f945cb2eb5e27578ed` plus diff non commité.

## Résultat en deux phrases

Une mission clôturée ne demande plus de reprendre une ancienne panne du
planificateur : son diagnostic reste consultable dans l'historique, tandis
qu'un blocage actuel et une preuve devenue périmée restent visibles.
Un lancement local à 12 appels ne remplace plus les limites communes de la
mission ; les plafonds et compteurs des tentatives déjà enregistrées sont conservés.

## Écart confirmé puis corrigé

Le rapport initial affirmait à tort que le garde de clôture couvrait déjà
`Planning.Failure`. Ce garde manquait dans `mission_status.go` et
`mission_guidance.go` ; le navigateur donnait aussi priorité à la panne et
à la pause avant la clôture. Les tests nouveaux ont échoué sur ces défauts
avant la correction, puis passé après correction.

Le démarrage manuel de T2 avait également mémorisé ses 12 appels et son délai
comme réglages communs. La reprise de T3 conservait ensuite les limites de sa
première tentative. Correction : conserver les limites communes déjà choisies,
et ne pas initialiser des plafonds communs depuis les plafonds locaux du premier
lancement. La protection qui borne une reprise par la tentative précédente
reste inchangée. Ce correctif ne donne pas rétroactivement 60 appels à T3.

## Matrice de preuves

| Critère | Observable et contrôle | Verdict de recette |
| --- | --- | --- |
| 1 / req-7 | Mission isolée acceptée avec preuves fraîches, responsable clôturé, ancien timeout et pause conservés : statut `termine`, action résultats ; `TestMissionClosedHistoryAndCurrentBlockCLI` et recette web | PASS |
| 2 / req-8 | Même fixture : histoire conservée sans mutation de l'échec, lecture CLI `mission changes`, ancienne panne repliable au clavier. Une dérive de preuve supprime le succès. Une panne actuelle reste une alerte avec action de reprise. | PASS |
| 3 / req-9 | Commande CLI réelle `mission status` FR/EN sur Store temporaire ; rendu réel Planning dans Chrome FR/EN × État/sombre, clavier Entrée, focus conservé, ouverture et repli de l'historique ; huit captures | PASS |

Ces fixtures n'appellent aucune IA. Leur avis favorable initial est une
précondition explicitement déclarée, non une démonstration d'autonomie réelle.
Le navigateur exécute les fonctions de rendu applicatives avec la sortie du
moteur isolé ; ce n'est pas une manipulation de la base de la mission réelle.
Les titres de mission et diagnostics fournis comme données ne sont pas traduits.

## Commandes et sorties

- `SWARM_QW6_EVIDENCE=/tmp/qw6-evidence go test ./... -run 'TestMissionClosedHistory|TestClosedPlanningKeeps|TestManualTaskLimitDoes' -count=1` : exit 0.
- `go test ./... -run 'TestFirstTaskLaunch|TestManualTaskLimitDoes|TestMissionClosedHistory|TestClosedPlanningKeeps' -count=1` : exit 0, 0.339 s (voir log exact).
- `go test -race ./... -run 'TestMissionClosedHistory|TestManualTaskLimitDoes|TestClosedPlanningKeeps' -count=1` : exit 0, 2.723 s.
- `go vet ./...` et `go build -o /tmp/swarm-qw6-candidate .` : exit 0.
- `npm test` et `node tests/states_contract_test.cjs` : exit 0.
- `node tests/qw6_closed_history_ui.cjs /tmp/qw6-evidence` : exit 0 ; aucune erreur console, exception de page ou requête en échec.
- `python3 tools/agent-workflows/check.py` : exit 0, neuf méthodes et contrat cohérents.
- `git diff --check` : exit 0.
- **Suite Go complète sur le candidat final : en cours lors de cette remise ; aucune acceptation finale avant son résultat.** Les contrôles moteur joints rejouent les scénarios ciblés avant la revue.

## Pièces explicitement jointes par les contrôles moteur

Manifest SHA256 et révision : `docs/plans/budgets-preuves-effort/T3-candidate.json`.
Recette : `T3-browser-result.json`, sortie moteur isolée `T3-closed-fixture.json`,
logs `T3-targeted-final.log`, `T3-race.log`, `T3-npm-final.log`, diff `T3-candidate.diff`
dans le même dossier. Captures dans `docs/screenshots/qw6/` :
`history-{fr,en}-{etat,sombre}.png` et `current-{fr,en}-{etat,sombre}.png`.
Le rapport initial partiel est conservé en `T3-worker-original.md`.

## Limites et prochaine étape

L'avis indépendant et l'acceptation restent à obtenir sur ces entrées liées.
La suite Go complète doit finir avant toute acceptation. Cette livraison ne
prouve ni une reprise autonome avec un fournisseur réel, ni le traitement de
l'échec du planificateur encore présent dans la mission actuelle ouverte.
Les preuves T2 liées à des fichiers partagés pourront nécessiter une actualisation
sur le candidat final ; elles ne sont pas déclarées fraîches par ce rapport.
Le moteur conserve les événements et les anciennes tentatives : aucun effacement.
