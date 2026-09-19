# P2 — Validation fondée sur des preuves

Date de la tentative : 2026-09-19.

## Résultat livré

Swarm expose maintenant une projection `evidence` unique, calculée à la lecture et utilisée par le JSON CLI, le snapshot/détail web et les vues textuelles CLI/web. Elle sépare explicitement :

- la revue du rapport (`report_review`) ;
- les contrôles effectivement lancés par le moteur (`controls`) ;
- la décision d’acceptation (`acceptance`).

La projection expose la tentative, la révision lue, la révision contrôlée lorsqu’elle est connue, la commande, le code de sortie, les dates de début et de fin, la fraîcheur et les limites. Une donnée historique non conservée est rendue par le code ou la valeur littérale `unknown`, jamais transformée en succès.

Les reçus automatiques nouvellement produits conservent en plus la commande réellement autorisée, la révision contrôlée, les dates et le fait que le processus du contrôle a effectivement démarré. Une commande dont le démarrage échoue est `not_executed`, et non `executed`. La sortie brute reste volontairement non exposée ; son empreinte SHA-256 est conservée, avec une limite de 65 536 octets explicitée.

Pour une gate humaine ou historique sans reçu moteur, les résultats et fichiers cités restent consultables, mais l’exécution est `unknown`, la commande est vide et le code de sortie est `null`. Une citation comme « npm test a réussi » ne démontre donc pas l’exécution de `npm test`.

La date et la révision propres d’une ancienne décision d’acceptation n’étaient pas persistées séparément de la gate : elles restent honnêtement `unknown`. La révision courante de lecture et, pour les nouveaux reçus automatiques, la révision contrôlée, sont disponibles séparément.

## Changements

- `evidence_projection.go` : contrat commun, fraîcheur recalculée, limites et rendu textuel.
- `model.go`, `automatic_validation.go` : reçu moteur enrichi sans lire de commande depuis un rapport ; distinction entre tentative de contrôle et processus effectivement démarré.
- `validation_state.go`, `web_server.go`, `review_dialog.go`, `view.go` : même projection dans `work show --json`, `/api/v1/snapshot`, `/api/v1/task`, la fiche CLI et le détail web.
- `evidence_contract_test.go` : régressions Go du rapport mensonger, de la preuve périmée, de la non-exécution, des métadonnées et de la parité CLI/web.
- `web/evidence-contract.js`, `tests/evidence_contract_test.cjs` : contrat de rendu DOM, échec sur état inconnu et affichage explicite des inconnues.
- `tests/audit_acceptance.cjs --case evidence` et script npm `test:audit:evidence` : dépendances obligatoires, assertions DOM réelles puis tests Go ciblés ; aucun succès fondé sur la seule présence d’un rapport.

## Preuves et commandes réellement exécutées

| Commande | Résultat observé | Portée / limite |
|---|---|---|
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm node tests/audit_acceptance.cjs --case evidence` | Premier passage en échec : `finished_at` était `unknown` pour un contrôle réellement terminé. | La régression a détecté un défaut réel de traçage ; le retour nommé Go a été corrigé. |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm node tests/audit_acceptance.cjs --case evidence` | PASS : DOM, tests Go et parité CLI/web. | Passage effectué avant le raffinement `not_executed`. |
| `npm test` | PASS : graphes, aperçu de mission, rafraîchissement cockpit et i18n. | Suite npm existante, sans navigateur réel. |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm go test -count=1 ./...` | Échec de préparation : cache Go par défaut en lecture seule. | Échec d’environnement, pas une preuve produit. |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p2-go-cache go test -count=1 ./...` | Échec : sockets Unix et TCP interdites (`operation not permitted`) dans les tests terminal et `httptest.NewServer`. | La suite Go complète n’est pas déclarée réussie dans ce bac à sable. Les échecs observés sont liés aux permissions réseau/socket. |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p2-go-cache go test -count=1 -run 'Test(EvidenceContract\|AutomaticValidation\|ValidationDrift\|HandoffCloses\|StateContract\|ResultPresentation\|WebTaskExposesOracleActions)' .` | PASS (`ok`, 0.708 s lors du premier passage ; 0.742 s sur la révision avec `not_executed`). | Tests Go appropriés sans écoute réseau réelle. |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p2-go-cache node tests/audit_acceptance.cjs --case evidence` | PASS final observé : `PASS evidence DOM`, `ok swarm.local/companion`, puis verdict ciblé. | Couvre rapport mensonger, preuve périmée, métadonnées, non-exécution et vérité CLI/web. |
| `git diff --check` | PASS, aucune sortie. | Vérification de forme ; ne démontre pas le comportement. |

## Correspondance critères / preuves / verdict

| Critère P2 | Preuve ciblée | Verdict | Limite |
|---|---|---|---|
| Revue du rapport distincte des contrôles et de l’acceptation | `TestEvidenceContractReportClaimIsNotExecutedControl` et rendu DOM | Prouvé sur les contrats testés | Une revue humaine informelle non enregistrée ne peut pas être datée. |
| Tentative, révision, commande, sortie, date, fraîcheur et limites exposées | `TestEvidenceContractExecutedControlAndStaleness` + assertion DOM | Prouvé pour les nouveaux reçus moteur | Les anciens reçus rendent les champs non persistés `unknown`. La sortie complète n’est pas exposée, seulement son empreinte. |
| `unknown` explicite | Régression rapport mensonger, rendu DOM et JSON CLI/web | Prouvé | `exit_code` inconnu est JSON `null`, accompagné de `execution: unknown`. |
| Une citation du rapport ne prouve pas un test | Rapport factice « npm test a réussi avec le code 0 », revue favorable, contrôle néanmoins `unknown` et acceptation `pending` | Prouvé | Le contenu sémantique général de tous les rapports n’est pas audité ; la règle structurelle est imposée. |
| Preuve périmée | Modification du rapport après acceptation automatique ; fraîcheur contrôle et acceptation passent à `stale` | Prouvé | La fraîcheur dépend des artefacts enregistrés dans la gate. |
| Même vérité CLI/web | Comparaison structurelle `reflect.DeepEqual` entre `work show --json` et `/api/v1/task`, plus détails textuels des deux voies | Prouvé | Aucun essai navigateur complet n’a été exécuté dans cette tentative. |

## Limites et risques résiduels

- La suite Go complète reste non prouvée dans ce bac à sable, qui interdit les écoutes Unix/TCP nécessaires à des tests sans lien direct avec P2. Les suites ciblées affectées passent.
- Les reçus déjà enregistrés avant ce changement n’ont ni date de début/fin ni indicateur `executed` fiable ; l’interface les présente donc comme `unknown`, même si leur ancien booléen `passed` existe.
- L’horodatage propre de l’acceptation humaine n’existe pas dans le modèle historique. Il n’est pas déduit de la date de gate.
- Le test DOM valide le contrat de rendu isolé. L’affichage web réel reçoit la même projection via `/api/v1/task` et son texte de revue, mais aucune recette navigateur visuelle n’a été lancée ici.

## Prochaine action recommandée

Sur un environnement autorisant les sockets locales, exécuter la suite Go complète inchangée :

```sh
TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p2-go-cache go test -count=1 ./...
```

Ne conclure au succès cumulatif global qu’après ce passage. Aucun push, déploiement, serveur utilisateur ou état Swarm n’a été modifié par cette tentative.
