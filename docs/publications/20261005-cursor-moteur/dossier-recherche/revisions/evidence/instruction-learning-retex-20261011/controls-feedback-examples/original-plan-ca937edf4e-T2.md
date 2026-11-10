# T2 — juge global et renouvellement : remise courante

Mission w-ca937edf4e0abd85f8d3d67b ; tâche plan-ca937edf4e-T2 ; producteur
native auto-a23c60f03a732c02841f, tentative a-2f9e1debb1548be96273f0d3 (3/3),
terminée. Base acceptée e32388fda21e7c225c3b2416d230d6f5450230cd.
Bilan local vérifié ; contrôles et revue indépendante de ce candidat requis avant
acceptation. Aucune autonomie réelle ou intégration native globale annoncée.

## Comportement livré et preuves

| Critère | Résultat local et preuve déclarée dans delivery.json |
| --- | --- |
| 1. Verdict global distinct de la revue | planning judge enregistre continue/end ; planning_judge.go et TestPlanningJudgeVerdictDistinctFromCandidateReview. |
| 2. Transition durable et contexte | planning renew après continue conserve objectif, exigences, tentatives, consommations et historique ; TestPlanningJudgeRenewalPreservesContextAndHistory et TestManagedCheckpointRecovery. |
| 3. Rejeu, fraîcheur, panne et quota | Tests TestPlanningJudgeReplayStaleAndDoubleVerdictRefused, TestPlanningJudgePausedAndQuotaRefuseEffects et TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess. Nouvelle suite Go entière brute exit0. |
| 4. Fin sous preuves et architecture | end appelle validatePlanningClosure ; TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs et TestPlanningJudgeEndRequiresFreshMatchingCandidate. Architectures FR/EN et contrat FR complets joints. Script documentaire structurel PASS ; conformité sémantique à revoir indépendamment. |

Les cinq fichiers Go concernés et score.go, les architectures complètes, le contrat,
les originaux ciblés et les reçus courants sont fournis sans troncature. Le contrôle
producteur renforcé inclut TestPlanningJudge et -v : 16 tests nommés PASS, exit0.
Les annonces d'outil publiques 14:13:29→14:14:21UTC sont des bornes observées,
sans prétendre reconstruire l'horodatage de commande non capturé. Le metadata
natif portant13:40 est conservé sous metadata.native-original.json ; le reçu
courant distingue cette incohérence historique et les preuves observées.

Nouvelle suite canonique non divisée : go test ./... -count=1 -timeout=12m,
11 octobre17:11:59→17:20:11UTC, exit0 en492,395s, paquet moteur470,557s.
Code Go identique à la copie native arrêtée ; manifeste1049sources vérifié
avant/après sans écart. Originaux exacts :
- docs/plans/engine-cursor-contract/evidence/T2-execution/operator-recovery-20261011/full-suite-canonical-20261011.out
- docs/plans/engine-cursor-contract/evidence/T2-execution/operator-recovery-20261011/full-suite-canonical-20261011-receipt.json

Tests locaux avec doubles et skips optionnels : ils ne prouvent ni les recettes
navigateur non exécutées ni l'autonomie d'un modèle réel. Les politiques Admin,
l'orchestration native complète et les sous-superviseurs restent T3–T7.

## Historique conservé et corrections attribuées

Interventions opérateur après la production : source alias/citations et portée
des tests corrigées, preuve brute canonique ajoutée ; aucun code Go, compteur,
ancien original ou avis indépendant modifié. Les résumés shardés anciens sont
historiques. Deux cardinalités égales ne prouvent ni authenticité ni fabrication ;
la nouvelle sortie brute constitue la preuve complète courante. Le refus1b5f est
conservé, aucun verdict annulé.

Le refus d77 signalait un chemin stdout absent. La vérification Git exacte du
candidat6c2da25/arbre201b3a8 démontre les deux originaux racine/imbriqué de même
SHA fbaa92ea28409f7d476cdf46ba7046372f7d868a050833c5c7c2d31872115199.
Le metadata courant nomme la copie imbriquée ; l'original natif reste inchangé.
Preuve : operator-recovery-20261011/path-binding-replan.json. Une absence du
seul fragment ne prouve aucune absence du candidat.

Le rapport précédent,22489octets, est conservé intégralement dans le commit
96c0ea89013b71656b4259ee9ee5a6a880f4de57 : docs/plan-ca937edf4e-T2.md,
SHA256 51e846248353e79e49f719e431b7d2df87965ae9228452fd4a80d586c5ee8aa7. Cette remise concise remplace l'accumulation
historique dans le contexte actif ; elle ne supprime aucun original du dépôt
Git. Précontrôle final trop grand observé sans nouvel appel à425 : cette
réduction volontaire est une nouvelle précondition, pas une troncature moteur.
Le détail historique demeure aussi dans l'index operator-recovery-20261011,
les metadata, anciens candidats et rapports. Les anciens contrôles filtrés ne
sont jamais déclarés comme la suite entière. Nouveaux contrôles/revue frais requis.
