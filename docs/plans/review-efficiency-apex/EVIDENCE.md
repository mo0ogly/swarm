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

## Deuxième lot — RD3 (remplace le statut « non terminé » ci-dessus)

| Exigence | Résultat | Preuve / limite |
|---|---|---|
| Réduction avec couverture complète | PASS | `TestManagedFragmentHistoricalSavingsAndIdentity` : 8 → 4 appels prévus, aucune preuve renommée sous le nouveau SHA |
| Entrée/dépendance modifiée | PASS | Groupe original entier invalidé ; contrat changé entraîne retour à la revue complète |
| Réserves et impacts obligatoires | PASS | Omission, citation historique seule, rapport seul, fichier seul et preuve inventée rejetés ; `unknown` interdit l’acceptation |
| Parcours public et persistance | PASS | `TestManagedFragmentHistoricalPublicRecovery/pass` : 6 → 4 appels réels au sous-processus simulé ; contrôles/revue même SHA, réouverture et rejeu sans dépense |
| Nouveau défaut | PASS | Même parcours `/fail` : une inspection, refus conservé, aucune publication |
| Corruption historique | PASS | Altération de l’ancien journal invalide la nouvelle preuve |
| Taille et budget connus avant appel | PASS | Prévol lecture seule inclut capacité finale et volume restant à inspecter |
| E6 débloquée | NON | 10 appels nécessaires, 7 disponibles, décision finale trop grande ; aucun appel réel lancé |

Tests ciblés : `go test -run 'TestManagedFragmentHistorical|TestReviewCost' -count=1 -v`
PASS 25.536s avant les derniers contrôles de petite modification/renommage et le
calcul exact de taille des observations historiques. Les résultats globaux finaux
seront consignés après leur exécution.

### Pourquoi le dossier E6 contient du bruit

Le contexte du dernier refus comporte 252 fichiers de diff : 107 moteur/autres,
84 tests, 20 interface, 41 documents/preuves. Les deux exports
`docs/e6-trial-originals/agents.json` et `snapshot.json` représentent environ
174 Ko de patch. Le candidat corrigé comporte maintenant 256 fichiers de diff,
mais seuls dix fichiers ont changé depuis ce refus.

Le contrat E6 demande un parcours réel refus → correction → acceptation. Son
dossier embarque également la revue des critères E1–E5 et les évolutions du moteur
accumulées depuis la dernière base acceptée. Ce n’est donc pas seulement une
répétition verbale du modèle : le moteur lui transmet effectivement un périmètre
beaucoup plus large. Supprimer les exports ou exclure arbitrairement des fichiers
ne démontrerait pas la conformité. La suite doit séparer revue des changements,
preuves d’exécution consultables et bilan final, avec une couverture attribuée
aux critères et des avis encore liés au candidat courant. Cette séparation n’est
pas implémentée par le simple cache d’observations RD3.

### Vérification finale du deuxième lot

- `go test ./...` : PASS 296.292s sur le code final. Le passage intermédiaire
  précédent était également réussi (303.268s).
- `go test -race -run 'TestManagedFragmentHistorical|TestReviewCostAvailableWithoutBudgetAndWithoutMutation|TestManagedFragmentStoreConcurrentReservationOnce' -count=1` : PASS 182.519s.
- `go test -run 'TestManagedFragmentHistoricalCapacity|TestManagedFragmentHistoricalTiny' -count=1` : PASS 0.539s ; petites modifications, renommages et conservation des questions dans le calcul de capacité.
- `go vet ./...`, `python3 tools/agent-workflows/check.py`, `git diff --check` : PASS.
- Aucune surface web modifiée ; aucun nouveau contrôle visuel revendiqué.
- Relecture effectuée par l’auteur avec la méthode code-reviewer, pas un avis
  indépendant de la mission. Recette verify-fix via l’entrée publique de reprise,
  transport sous-processus simulé, état relu après réouverture.

## Protocole 4 — recette finale

- Tests ciblés admission/protocole/Store : PASS 17,796 s.
- `go test ./...` : PASS 322,856 s sur le code final.
- `go test -race -run 'TestManagedFragmentToken|TestObservedReviewInput|TestReviewInputTokens' -count=1` : PASS 166,086 s.
- `go vet ./...`, contrôle du contrat agent, `git diff --check` : PASS.
- Prévol CLI du candidat E6 existant avec le client natif configuré : six appels
  requis pour sept disponibles, transport prêt, état de mission identique avant
  et après ; zéro appel IA. Les cinq groupes historiques et 275 pièces restent
  présents. Résultat enregistré dans `v4-native-client-cost.json` des artefacts
  de livraison, mesures/tests dans `v4-checks.json`.
- Relecture par l’auteur : découverte et correction du cache provenant d’un autre
  client, du faux refus sur JSON échappé et de la divergence prévol/exécution.
  Aucun avis indépendant de la mission revendiqué pour cette relecture.
- Pas de changement d’interface ni validation visuelle nouvelle revendiquée.
- E6 n’est pas acceptée par ces tests : la revue indépendante doit encore aboutir.

### Livraison et reprise effective — 2026-09-27T15:06:24.675156+00:00

Commit moteur a0780d62f95e3e7c70e1be8e214d7b9bb214f447 installé, PID1649530,
SHA256 f98c84adb1bd3fb8871d04ab00cbb0ba7ea8e641838698d457bae5859a2de3b2.
Santé prête, état conservé pendant installation, huit assets web identiques.
CLI et HTTP installés donnent le même prévol6/7 sans mutation. Mission reprise.
Action publique retry-review enregistrée sous retry-e6-token-protocol4-20260927.
Revue réelle E6 démarrée : review-60fafd58dc2f40ba6541fc97, état running,
révision461, premier appel réservé, compteur65/71. Aucun nouveau producteur,
même candidat et mêmes contrôles. Ce n’est PAS encore une acceptation.
Le statut global de la tâche et son ancien motif restent blocked pendant cette
revue ; lire independent_review.state pour l’activité réelle, ne pas relancer en
parallèle à cause de cette ancienne étiquette. À suivre comme point de lisibilité.
