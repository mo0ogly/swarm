# Vérification — premier lot APEX

## Couverture

| Exigence | Résultat | Preuve / limite |
|---|---|---|
| Refus détaillé au premier appel | PASS | Fournisseur simulé via sous-processus : citation/ligne/explication/reproduction/attendu conservés ; un appel avant refus |
| Citation inventée, ligne incorrecte, justification vide | PASS | TestFragmentV2ActionableRefusal refuse ces entrées |
| Lecture historique | PASS | Schéma v1 inchangé ; nouveaux plans v2 ; suite historique complète |
| Prévol sans dépense, CLI et API cohérents | PASS | TestReviewCostAvailableWithoutBudgetAndWithoutMutation, budget zéro, aucun compteur ni état modifié |
| Deux interruptions identiques | PASS | TestRepeatedFragmentRetryStopsBeforeMutation : refus avant mutation, compteur conservé |
| Réutilisation entre candidats | NON TERMINÉ | Contrat et ordre de mise en œuvre dans DIFFERENTIAL.md ; zéro cache automatique actif |
| Réduction du coût E6 | NON DÉMONTRÉE | Prévol réel en lecture seule :275pièces,256fichiers de diff,10modifiés depuis refus,13+2appels,7disponibles |
| Acceptation E6 | NON | Aucun nouvel appel réel, aucun changement d'acceptation |

## Commandes exécutées

- `go test ./...` : PASS 433.096s, source de production finale. Un premier passage sur une version antérieure a été interrompu pour diagnostic ; il n'est pas compté comme succès.
- `go vet ./...` : PASS.
- `go test -race -run 'TestRepeatedFragmentRetryStopsBeforeMutation|TestReviewCostAvailableWithoutBudgetAndWithoutMutation|TestManagedFragmentStoreConcurrentReservationOnce|TestManagedFragmentRefusalRequiresOriginalProof' -count=1` : PASS11.088s.
- `go test -run 'TestFragmentV2|TestManagedFragmentRuntimeProvider|TestReviewCost|TestRepeatedFragment' -count=1` : PASS31.749s.
- `go test -run 'TestRepeatedFragment|TestManagedFragmentRefusal|TestFragmentDefectSource' -count=1` : PASS3.978s, inclut le test transactionnel supplémentaire.
- `python3 tools/agent-workflows/check.py` : PASS, huit méthodes partagées.
- `git diff --check` : PASS.

Tous les tests fournisseur ci-dessus emploient des doubles déterministes. Ce ne sont pas des avis indépendants sur la mission ni des mesures de qualité d'un vrai modèle. Les logs complets et le prévol live en lecture seule sont conservés hors dépôt dans les artefacts de la session.
