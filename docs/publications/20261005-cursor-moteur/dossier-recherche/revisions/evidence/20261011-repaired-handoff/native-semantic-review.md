# Rapport de tentative — plan-ca937edf4e-T2 (a-5fff6319655d0b5b9b34413a, départ 1/2)

Racine de travail : `/home/fpizzi/workspace/swarm/.swarm/managed/w-ca937edf4e0abd85f8d3d67b/copies/plan-ca937edf4e-T2-1`.
Base candidate : `e32388fda21e7c225c3b2416d230d6f5450230cd`. Livrable complet et matrice de
preuves détaillée : [`docs/plans/engine-cursor-contract/evidence/T2-handoff.md`](plans/engine-cursor-contract/evidence/T2-handoff.md).
Ce fichier est le rapport unique de cette tentative pour le relais automatique.

## Résultat en deux phrases

Juge global (`judge-verdict` : continue/finish/blocked, distinct de la revue
indépendante de candidat) et renouvellement de cycle (`Cycle`, baux révoqués,
événement `cycle_renewed`, objectif/exigences/tâches/tentatives/limites
conservés) implémentés dans `internal/engine/planning_judge.go` et câblés dans
`planning.go`, avec rejeu/péremption/panne/quota refusant l'effet avant toute
mutation ; 13 tests Go `PASS`, aucune régression observée sur la suite complète
du paquet sauf un échec préexistant hors périmètre (`AGENTS.md`, sans lien avec
ce diff). Non testé ce lot : recette avec fournisseur réel, déclenchement
périodique automatique, mission longue multi-cycles réelle.

## Critères — statut

| req | Intitulé | Statut | Preuve |
| --- | --- | --- | --- |
| req-5 | Verdict global distinct de la revue de candidat | PASS | `TestJudgeVerdictDistinctFromIndependentReview` |
| req-6 | Transition durable, contexte reconstruit, état conservé | PASS | `TestJudgeVerdictContinuePreservesStateAndRenewsCycle` |
| req-7 | Rejeu/péremption/panne/quota refusent l'effet ou l'appel double | PASS | `TestJudgeVerdictReplayStaleQuotaFailureRefuseEffects` (5 sous-cas), `TestJudgePeriodGatesClaimUntilVerdictRendered` |
| req-8 | Aucune clôture globale sans preuve fraîche ; architecture documentée | PASS | `TestJudgeVerdictFinishRequiresRootClosedAndFreshEvidence` (2 sous-cas) ; sections « Lot T2 » ajoutées aux 4 documents FR/EN |

Détail des commandes, codes de sortie et table complète cible/vérifié/limite
dans le rapport lié ci-dessus (section « Méthode de vérification locale » et
« Couverture des critères assignés »).

## Commandes rejouées cette tentative (résumé)

| Commande | Résultat | Code |
| --- | --- | --- |
| `go build ./internal/engine/...` | aucune sortie | 0 |
| `go vet ./internal/engine/...` | aucune sortie | 0 |
| `go test ./internal/engine -run 'TestJudge\|TestConfigureJudge' -count=1 -timeout 60s -v` | 13 passed | 0 |
| `go test ./internal/engine -run 'TestPreparedOrganizationManagedRepository\|TestPlanningParent\|TestPlanningReportRelay\|TestManagedCheckpointRecovery\|TestManagedRecoveryHandoff' -count=1 -timeout 25s` (contrôle `plan-validation`) | 16 passed | 0 |
| `go test ./internal/engine -count=1 -timeout 280s` (suite complète, non-régression large, hors contrôles configurés) | 1017 passed, 1 failed (`TestGraphDeliveryD03QualificationManifest`, préexistant, hors périmètre), 5 skipped | 1 |
| `python3 docs/plans/engine-cursor-contract/check-evolution-docs.py T2` (contrôles `plan-entry`/`plan-delivery`/`plan-criterion-1..4`) | `T2: architecture/evidence structure verified; semantic review still required` | 0 |

## Changements

- `internal/engine/planning.go` : champs `Judge`/`Cycle`/`LastJudgeDecisions`/`JudgeCalls`/`Finished`/`Verdicts` sur `PlanningState` ; dispatch `configure-judge`/`judge-verdict` ; garde `Finished` sur `claim`/`decide` ; garde `judge_due` sur `claim` ; bornes ajoutées à `validatePlanningState`.
- `internal/engine/planning_judge.go` (nouveau) : `JudgePolicy`, `JudgeVerdict`, `configureJudge`, `renderJudgeVerdict`, `judgeMissingProofs`.
- `internal/engine/planning_judge_test.go` (nouveau) : 13 cas de test comportementaux.
- `docs/architecture/CURSOR-ENGINE-CONTRACT.md`, `docs/en/CURSOR-ENGINE-CONTRACT.md` : lignes JAN-4/JAN-5/FEB-3 et deux nouveaux invariants mis à jour.
- `docs/architecture/CURSOR-ENGINE-EVOLUTION-20261010.md`, `docs/en/CURSOR-ENGINE-EVOLUTION-20261010.md` : section « Lot T2 » ajoutée (états, identités, persistance, rejeu, preuves).

## Limites

- Aucune migration SQL nouvelle : choix documenté (JSON additionnel sur `PlanningState`, déjà dans la transaction atomique existante), pas une omission.
- Mode `automatic` de `JudgePolicy` persisté mais non déclenché automatiquement par ce lot (cible documentée, pas comportement vérifié).
- `TestGraphDeliveryD03QualificationManifest` reste en échec, cause (`AGENTS.md`) étrangère à ce diff (`git diff --stat` ne touche ni `AGENTS.md` ni les fichiers de graphe) ; conservé visible, non falsifié, non corrigé (hors périmètre `internal/engine/planning*.go`).
- Interface Admin web (FR/EN) pour `JudgePolicy` : hors périmètre explicite de T2 (prévue T5 selon le tableau des livrables de `CURSOR-ENGINE-EVOLUTION-20261010.md`) ; l'exposition en commande publique existe dès ce lot via le dispatch générique déjà testé (`planning configure-judge|judge-verdict WORK --input requête.json`, CLI et `/api/v1/planning?action=...`).
- Ce bilan est une vérification locale du producteur ; il ne vaut pas acceptation. Revue indépendante du même candidat requise avant toute acceptation par le moteur.

## Prochaine action

Responsable : lancer la revue indépendante sur ce même candidat (base
`e32388fda21e7c225c3b2416d230d6f5450230cd` + diff listé ci-dessus), puis, si
acceptée, T3 (exploration en lecture, périmètres, concurrence, réservations et
budgets partagés) et T5 (réglages persistés, Admin FR/EN de `JudgePolicy`).


## Correction externe du dossier de preuve — 10 octobre 2026

Les résultats de tests annoncés plus haut restent des déclarations historiques
du producteur. Les originaux de ces exécutions n’étaient pas joints au dossier ;
ils ne sont pas réattribués au contrôleur ni reconstruits rétroactivement.
Une reproduction opérateur, distincte, est conservée dans
`docs/plans/engine-cursor-contract/evidence/T2-execution/index.json` avec les
arguments, dates, codes et stdout/stderr originaux, et les blobs de code testés.

Sur le candidat source `a27e175eb4f3e38f9cede1c888956a5ca5a0310f`,
`go test ./internal/engine -run ^Test(Judge|ConfigureJudge) -json -count=1 -timeout 60s`
retourne 0 : six tests de premier niveau, 13 PASS avec sous-cas. Le code du juge
et ses tests sont inchangés par cette réparation du dossier. L’origine est
opérateur en clone isolé ; cette preuve ne constitue pas un reçu moteur.

La suite `go test ./... -json -count=1 -timeout 15m` s’interrompt avec code du
lanceur 143 et code Go inconnu. Ses traces partielles ont 979 PASS, huit SKIP
et un FAIL de premier niveau (`TestGraphDeliveryD03QualificationManifest`).
Aucun bilan de suite complète n’est établi par cette reproduction. Les originaux
volumineux restent hors du candidat, avec chemin et empreinte ; l’extrait joint
de l’échec est explicitement borné et ne remplace pas leur intégralité.
La cause de l’interruption reste inconnue.

Cette réparation apporte une précondition nouvelle : des originaux attribués
réellement disponibles pour juger les six tests, au lieu d’une déclaration seule.
Nouveaux contrôles et revue du candidat corrigé restent requis. Aucun critère
n’est accepté par cette note, aucune tentative ou consommation n’est remboursée.

Sélection des sources jointes : originaux du juge, observation de suite partielle, événements du manifeste et sources FR complets. Les ajouts des deux miroirs EN restent dans le diff candidat ; leur texte complet préexistant est couvert par T1. Aucun critère ni code n’est supprimé pour tenir la capacité de contexte.