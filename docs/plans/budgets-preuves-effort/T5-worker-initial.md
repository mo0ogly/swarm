# Rapport — T5 (QW7 : voir l'effort par tâche)

Tentative a-58b415b09e3effca87fb4a92 (départ 1/2). Révision du travail au départ : 89.
Racine de travail : `/home/fpizzi/workspace/swarm-action-skills` (vérifiée par `pwd` +
`git rev-parse --show-toplevel`).

Statut : EN COURS — exploration initiale terminée, implémentation en cours.

## Critères assignés (non encore vérifiés à ce stade)

- req-13 : Mesures datées concordent avec traces et distinguent
  processus/remises/revues/événements. — NON VÉRIFIÉ
- req-14 : Mesures absentes non rapportées, sans coût zéro ou jetons inventés ;
  tentatives distinguées. — NON VÉRIFIÉ
- req-15 : Résumé lisible FR/EN, deux thèmes, clavier/focus, CLI isolé. — NON VÉRIFIÉ

## Constat d'exploration

- `mission_insights.go` contient déjà `SpendingRow`, `AttemptLedger`,
  `missionSpending`, `attemptLedgers`, `printMissionSpending`,
  `printAttemptLedgers` (QW3/QW4 antérieurs). Affichage web dans
  `web/mission-insights.js` (`spendingView`, `attemptsView`).
- Champs par tentative déjà présents : `Calls`, `Tools`, `Reads`, `Writes`,
  `Unclassified`, `Errors`, `Repeats`, `Tests`, `Degraded`, `Input`, `Output`,
  `MissingUsage`, `Cost`, `ProcessState`, `Accepted`.
- Manque identifié (portée T5, pas de refonte) : aucune date/provenance sur
  `AttemptLedger` (durée de tentative), aucun compteur de revues ni de reprises
  par tâche dans `SpendingRow` (seul `MissionSpending.Retries` est global, non
  distingué par tâche).
- `Agent.Started` / `Agent.Ended` (agents_store.go) et
  `IndependentReview.Started/Finished` (independent_review.go) existent déjà et
  sont fiables pour dater sans invention.
- `Task.PreviousReviews` + `Task.IndependentReview` permettent de compter les
  revues réellement enregistrées par tâche.

## Prochaine action

Ajouter uniquement : `Started`/`Ended` sur `AttemptLedger` ; `Reviews` et
`TaskRetries` sur `SpendingRow` (ligne `worker`), calculés depuis les données
déjà stockées (pas de nouvel inventaire). Mettre à jour l'affichage CLI
(`printAttemptLedgers`, `printMissionSpending`) et web
(`web/mission-insights.js`), FR (`uiText`) et EN (`locales/en.json`
/ `web/i18n-en.js`). Ajouter tests ciblés Go. Vérifier FR/EN, deux thèmes,
clavier/focus en navigateur réel, et CLI sur racine isolée.

## Limites à ce stade

Rapport initial écrit avant implémentation, conformément au cadre d'exécution.
Sera mis à jour après résultats utiles et au plus tard avant l'appel 48.
