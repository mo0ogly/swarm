# E7 — requalification de l'historique

## Vérification du 23 septembre 2026

`node tests/engine_acceptance.cjs --case history` : **PASS**, 6 cas/sous-cas,
4,352 s. Tests sur Stores temporaires ; aucune base de mission réelle modifiée.

| Exigence | Preuve | Résultat |
|---|---|---|
| Une acceptation ancienne sans reviewer n'est plus une preuve fraîche | `TestEngineContractHistoryLegacyAcceptanceReadOnly`, lecteur moteur et CLI `work show` | PASS |
| L'ancien statut reste consultable ; lecture sans réécriture | Corps persisté identique avant/après deux lectures CLI | PASS |
| Reprendre un résultat conservé sans recréer la production | `TestIntegrationRetryPublicPreservesResultAndReviews` | PASS, cas d'intégration admissible |
| Ne pas inventer une ancienne sortie manquante | `TestIntegrationRetryLegacyKeepsMissingEvidenceExplicit` | PASS |
| Une nouvelle révision exige une nouvelle preuve | Tests de révision cumulative et reçus historiques | PASS |

Les revues de ces tests utilisent des fournisseurs déterministes. Ce résultat
ne constitue pas un avis IA indépendant sur cette livraison.

## Parcours public et limites

1. Lire `swarm --json work show WORK` sur une copie isolée : distinguer le statut
   historique de `validation.tasks[TASK].fresh`.
2. Conserver événements, reçus, candidat et identités de tentative ; ne pas
   réécrire l'ancienne acceptation ni inventer les pièces absentes.
3. Si le cas est un échec d'intégration admissible, utiliser
   `planning retry-integration WORK --input request.json` selon le contrat de
   [reprise](REVIEW-TIMEOUT.md#reprendre-une-intégration-après-un-contrôle-en-échec).
   Cette opération relance contrôles et revue sur le nouveau candidat ; une
   réservation ne vaut jamais acceptation.
4. Une ancienne acceptation sans reviewer n'est **pas automatiquement** un cas
   admissible à `retry-integration`. Il manque encore une procédure générale de
   requalification de ce cas, avec conservation du candidat et revue explicite.

**E7 reste partielle** : refus rétrospectif et conservation prouvés ; procédure
universelle de remise en revue des anciennes acceptations et avis indépendant
sur cette livraison encore absents. Ne pas afficher cette étape comme terminée.
