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
4. Pour une ancienne acceptation **Git gérée**, sans avis indépendant, utiliser
   `planning requalify WORK --input request.json`. La demande contient
   `schema_version`, `event_id`, `expected_revision`, `task_id`, `agent_id`,
   `attempt_id`, `result_commit`, `expected_candidate` et un `reason` explicite.
   Relever ces identités dans l'état public ; ne pas les deviner.

La requalification exige un producteur terminé, un résultat intégré conservé,
un vérificateur configuré avec budget disponible, des contrôles complets et
aucun agent, décision ou avis en cours. Elle retire la validité courante et
archive l'ancien statut, gate et reçu dans `requalifications`. Le conducteur
reprend ensuite le résultat sans nouvelle production, relance les contrôles
cumulatifs et demande une revue indépendante. Le dossier de preuves et
l'événement de publication sont distincts des anciens ; aucune pièce n'est
écrasée. La même demande est idempotente, y compris après publication.

Une dérive de candidat ou de contrat empêche la reprise. Une revue existante,
un résultat réparé, un budget épuisé ou une provenance absente est refusé :
les parcours de récupération spécialisés restent nécessaires. Les missions
sans dépôt Git géré ne sont pas couvertes par cette commande.

## Portée de la qualification

Le scénario `TestEngineContractHistoryPublicRequalification` couvre réservation,
redémarrage, nouveaux contrôles/revue, publication, ancien reçu inchangé et rejeu.
Les cas de refus couvrent révision, candidat, résultat, tentative, vérificateur,
budget et avis déjà présent. Fournisseur déterministe : ce test ne vaut pas une
revue IA indépendante de la livraison. La mission principale reste inchangée.
