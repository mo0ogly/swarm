# P1 — Un verdict cohérent partout

## Résultat livré

Le verdict de mission est désormais un contrat à deux champs : `validation.state` est un code métier invariant (`validated`, `waived`, `blocked`, `abandoned`, `partial`, `open`) et `validation.label` est un libellé d'affichage. Le JSON de `work show` expose le même objet dérivé que le snapshot web. La CLI humaine continue d'afficher le libellé FR ou EN.

Le cockpit ne compare plus `validation.state` à `VALIDÉ` traduit. `web/status-contract.js`, chargé avant le cockpit et la planification, refuse explicitement un code inconnu et centralise les décisions d'affichage.

La planification expose dans le DOM deux dimensions distinctes : `data-review-availability="absent|configured"` et `data-review-state="absent|idle|running|paused|error|recorded"`. Un vérificateur configuré sans avis affiche maintenant « Aucun avis pour le moment » au lieu d'annoncer un résultat à examiner. Les états de planification `active`, `completed`, `paused` et `error` sont eux aussi dérivés de codes, jamais de textes traduits.

Les traductions anglaises ont été ajoutées à `locales/en.json`, puis `web/i18n-en.js` a été régénéré. Aucun jeton de thème ni aucune donnée utilisateur n'a été modifié.

## Preuves et résultats

| Exigence | Preuve exécutée | Verdict | Limite |
|---|---|---|---|
| État métier indépendant de la langue | `TestStateContractUsesInvariantCodesAndSeparateLabels` compare FR/EN : code `validated` identique, libellés `VALIDÉ`/`VALIDATED` distincts | PASS | Deux langues prises en charge par le produit |
| Même vérité CLI JSON / web | Le même test appelle réellement `work show --json` et `cockpitSnapshot`, puis compare `state` et `label` | PASS | Snapshot local, sans serveur HTTP |
| Terminé et partiel | Tests Go `validated` et `partial`; test DOM `completed` et résumé validé/partiel | PASS | Pas de capture graphique, le DOM est isolé en mémoire |
| Preuve périmée | Modification réelle de `proof.txt`, puis assertions `blocked`, compteur `stale=1` et tâche `stale` | PASS | Fraîcheur couverte sur artefact local |
| Erreur et pause | Contrat DOM `planning()` et `review()` avec assertions `error` et `paused` | PASS | Aucun fournisseur réel appelé |
| Revue absente/configurée | Exécution réelle de `Planning.roles` sur un DOM isolé ; assertions sur les attributs et absence du faux texte « Attend un résultat à examiner » | PASS | Revue IA non lancée, conformément au périmètre déterministe |
| Cas inconnu / dépendance absente | `states.validation()` rejette `VALIDÉ` comme code et rejette `future-state`; `audit_acceptance.cjs` vérifie chaque fichier requis avant les assertions | PASS | — |
| Catalogue FR/EN cohérent | `npm test` inclut la parité de `locales/en.json` et `web/i18n-en.js` | PASS | — |

## Commandes réellement exécutées

- `npm run i18n:build` — PASS.
- `node tests/states_contract_test.cjs` — PASS : verdicts invariants, FR/EN, terminé, partiel, preuve périmée, erreur, pause et revue absente/configurée.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p1-go-cache go test -count=1 -run '^TestStateContract' .` — PASS.
- `npm run test:audit:states` — PASS. Dans ce sandbox, le cas Node exécute les assertions DOM dans le même processus, car la création d'un sous-processus depuis Node est refusée (`EPERM`). Le contrat Go/JSON est donc exécuté par la commande Go distincte ci-dessus ; le lanceur l'indique explicitement et ne prétend pas l'avoir exécuté.
- `npm test` — PASS : graphes, aperçu de mission, rafraîchissement cockpit et i18n.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p1-go-cache go test -count=1 -run '^(TestStateContract|TestValidationDriftBlocksAckAndDependencies|TestHandoffClosesAfterFreshAcceptance|TestAcceptanceFreshnessAndAttempts|TestAbandonedIsNotCompletion|TestSharedRequestReplayAndConflict|TestCockpit)' .` — PASS.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p1-go-cache go test -count=1 ./...` — ÉCHEC externe au lot : le sandbox refuse les sockets Unix des tests terminal (`setsockopt: operation not permitted`) puis l'écoute TCP de `httptest` (`listen tcp6 [::1]:0: socket: operation not permitted`). Les tests P1 ciblés ont ensuite été exécutés et passent.
- `git diff --check` — PASS sur la révision finale de cette tentative.

## Limites et prochaine action

Le rendu a été testé par construction DOM isolée dans les deux langues et n'introduit aucun style ; aucun navigateur avec serveur HTTP n'a pu être lancé dans ce sandbox sans droits réseau. La suite Go complète reste non concluante ici pour la même restriction de sockets, et ce rapport ne la présente pas comme validée.

Prochaine action du responsable : intégrer la révision, exécuter `npm run test:audit:states`, le test Go `^TestStateContract`, `npm test`, puis la suite Go complète dans un environnement autorisant les sockets locales. Vérifier ensuite les attributs `data-validation-state`, `data-planning-state`, `data-review-availability` et `data-review-state` dans la recette navigateur cumulative FR/EN et thèmes État/sombre.
