# Revue indépendante T7 — candidat 44a86e6

**Verdict : request changes. Deux findings P2 confirmés. Aucune acceptation native.**

Revue indépendante autorisée, en contexte séparé, lecture seule du candidat `/home/fpizzi/Documents/ChatGPT/swarm`, HEAD `44a86e646c37da503a38e4a687aeb1a50ecb19e1`, checkout propre au début et à la fin. Base documentaire de l'évolution : `1169df70bec73d50fd23f640245b8ffd1317b9de`. Aucun fichier du candidat modifié, aucun autre agent appelé, aucune inférence réelle lancée par ce reviewer, aucune mission historique ou base native privée utilisée, aucun service partagé interrompu. Rapport unique hors checkout.

Méthodes : `tools/agent-workflows/CONTRACT.md`, `.agents/skills/code-reviewer/SKILL.md` et méthode personnelle `replan`. Besoin récupéré depuis AGENTS.md, completion-audit-20261012.md, evolution-20261010.json et les huit rapports T1–T8. Les reçus, états publics et plans sont distingués du comportement.

## Findings

### F1 — [P2] Valider le même historique dans preview et apply

Localisation : `internal/engine/web_requests_config.go:159–167`, spécialement le bloc `if !preview` à la ligne 159.

La sérialisation et la limite de stockage de 1 Mio sont contrôlées seulement pour l'application. À historique proche de la borne, la demande publique `preview` réussit et annonce la révision proposée alors que la même demande `apply` est refusée. La modale Admin utilise exactement ces opérations ; elle affiche donc une proposition que le moteur sait ne pas pouvoir enregistrer. T5 demande une validation et un aperçu/application cohérents.

Reproduction indépendante terminée via CLI publique du binaire figé : initialiser une racine temporaire ; appliquer 418 changements `web-requests` avec des motifs ASCII de 2 000 caractères ; proposer le changement suivant avec `expected_revision=418`. `preview` sort 0 et annonce la révision 419 ; `apply` sort 2 avec `Historique plein / History full`. Une relecture publique reste valide à la révision 418. L'original ci-dessous inclut demande, sorties, révision et identité du binaire ; fixture conservée `/tmp/swarm-t7-review-web-history-nfo39t5w`.

Correction requise : construire/sérialiser et valider les octets réellement écrits avant la branche de persistance, y compris le saut de ligne final, pour preview et apply. La garde actuelle `len(raw)>1048576` omet aussi le saut de ligne ajouté : au cas exact `len(raw)==1048576`, l'écriture produirait un fichier refusé à la prochaine lecture. Cette conséquence de bord découle directement du producteur/lecteur ; seul le désaccord à historique plein est reproduit dynamiquement ici. Garder une régression de refus sans mutation, taille exacte et relecture, comme pour GraphPerformance.

### F2 — [P2] Conserver une cause fournisseur exploitable pour les événements Codex en échec

Localisations : `internal/engine/assist_run.go:231–239` (extraction) et `internal/engine/planning_runner.go:479–483` (propagation).

Pour un événement Codex `{"type":"error","message":"…"}`, le parseur lit seulement `result`/`error` et fabrique `<nil> <nil>`. `turn.failed.error.message` n'est utilisé que pour le cas spécial `invalid_json_schema`. Sur un exit non nul, le runner écarte ensuite `result.err` sauf ce cas spécial et ne garde que code de sortie, compteurs et stderr. Un refus réel de capacité écrit sur stdout avec stderr vide perd donc sa cause dans la panne durable remise au superviseur ; ce dernier reçoit un échec opaque et ne peut distinguer disponibilité, schéma, compte ou autre refus à partir de son contexte courant.

Original opérateur vérifié en lecture seule, sans nouvel appel reviewer : `t7-provider-structured-original.json` contient exit 1, stderr vide et exactement ces événements :

```json
{"type":"error","message":"Selected model is at capacity. Please try a different model."}
{"type":"turn.failed","error":{"message":"Selected model is at capacity. Please try a different model."}}
```

Le `current.json` public de la fixture de recette conserve `planning.failure` avec `exit status 1`, 531 octets, 5 événements, `dernier type=turn.failed`, mais aucune cause de capacité ; `activations=1`, `decisions=0`. La source examinée explique précisément cette perte. Cette exécution réelle appartient à l'opérateur ; ma confirmation indépendante porte sur l'original et le chemin source, pas sur une nouvelle inférence. Le probe sans schéma sorti 0 n'établit pas que le schéma est la cause du refus ou que le fournisseur est désormais disponible.

Correction requise : parser les variantes structurées du contrat d'erreur, conserver une classification et un diagnostic sûr/borné sans copier secrets ou payload arbitraire, puis propager cette cause avec l'exit non nul. Aucun fallback de modèle, attente inventée, relance inchangée ni remise à zéro. Régressions locales nécessaires pour error.message, turn.failed.error.message, exit 0/1, erreurs malformées et absence de fuite ; préserver le refus indépendant du fournisseur.

## Critères couverts et limites

| Contrat | Examen et preuve | Résultat de revue |
| --- | --- | --- |
| Jugement/renouvellement frais T2 | planning_judge.go, planning.go, planning_schema.go, planning_context.go, planning_delivery.go ; cycle/schéma reçu, rejeu, stale, pause, fin avec preuves et processus actif ; tests reviewer -race | PASS local sur chemins testés ; qualité du jugement réel non démontrée |
| Fin métier root | validatePlanningClosure, opération judge end, scopes descendants, acceptedFresh ; contrôle non-clôture automatique après acceptation | PASS local ; les chemins legacy close/public judge gardent leur contrat, pas d'équivalence native inventée |
| Délégation/sous-planification T3 | planning.go, planning_runner.go, planning_budget.go, planning_exploration.go, mission.go ; deux subprocess doubles simultanés, budget partagé, pause/réouverture, discovery persistée et stale refusée | PASS local ; aucun sous-planificateur IA réel observé par reviewer |
| Staging/publication isolés T4 | managed_result_staging.go, managed_integration.go, managed_publication_lane.go, managed_review.go ; résultat immuable, publication exclusive, revalidation du candidat/contrat/reçu, staging sœur sans review payée, identité/pauses | PASS local sur chemins testés ; serveur natif non migré |
| Reprise et appartenance après relocation | model.go, managed_git.go, managed_delivery.go, managed_preparation.go, agent_exchange.go ; alias racine, sous-dossier Git, copie réutilisée, refus liens enfant/copieuse/metadata ; baux expirés ou remplacés non ressuscités | PASS local ; relocation physique de fixtures uniquement |
| Admin API/CLI T5 | quotas.go, web_requests_config.go, graph_performance_config.go et consommateurs ; preview/replay/CAS/history, mêmes valeurs et conservation consommation | PARTIAL : F1 confirmé ; F2 bloque le diagnostic de recette réelle |
| UX graph studio | graph-studio.js, graph-draft.js, sources et recette archivées, deux screenshots FR sombre/EN état inspectés ; hashes graphe identiques au reçu ; tests Node de géométrie et refus/sauvegarde | PASS des contrôles locaux exécutés et des preuves examinées ; aucune nouvelle recette navigateur exécutée par reviewer, aucun audit complet contraste/lecteur écran |
| Politique performance | valeurs et bornes exactes Go/JS, octets transport exposés, Admin et adaptateur réellement généré ; strict/relaxed/equal/custom workload ; vérification -race de preview oversized/no mutation | PASS local sur chemins testés ; mesures 50/200/500 historiques sur binaires identifiés, pas nouvelles mesures reviewer |
| T7/T8 cumulatif | dossiers/limites et refus réel original inspectés | NOT COMPLETE : refus capacité réel, critères autonomes cumulatifs non observés ; aucun coût/tokens/cache réel déduit, aucun RETEX final ou acceptation native établi |

## Contrôles exécutés par le reviewer

- Go ciblé en `-race` : 32 tests de premier niveau / 68 avec sous-cas, puis 9 / 9 sur attentes, quotas et baux ; **41 / 77 au total, exit 0**, noms et sorties originales ci-dessous. Doubles déterministes et racines temporaires des tests uniquement. Pas de suite entière.
- `node tests/graph_studio_test.cjs` : PASS ; LR/TB, brouillon refusé conservé, aperçu antérieur invalide, sauvegarde corrigée rétablit la disponibilité.
- `node tests/graph_performance_adapter_test.cjs` : PASS ; verdict du code généré suit les cibles strictes/souples/égales et charge personnalisée.
- `node tests/web_requests_test.cjs` : PASS ; valeurs reçues, corps complet, zéro, longs délais par chunks, annulation/nettoyage, pas de replay, configuration absente/invalide refusée.
- `node tests/asset_loading_policy_test.cjs` : PASS ; délai Admin, zéro, nettoyage et longs délais éditeur.
- `python3 tools/agent-workflows/check.py` : exit 0, `OK: 12 shared Claude/Codex skills, preparation commands, contract and engine-built role contexts`.
- `git diff --check` : exit 0 ; checkout propre et HEAD inchangé confirmé après tous les tests.
- Reproducer CLI F1 : FAIL du contrat preview/apply, résultat original préservé. Aucun test vert ne l'annule.

Les reçus historiques sont réutilisés seulement pour leurs sources pertinentes inchangées. Les hashes `graph-studio.js`, `graph-draft.js`, éditeur, CSS et recette du graphe correspondent ; i18n a un ajout ultérieur explicatif sur les bornes de performance, visible dans le diff (pas une modification du parcours graphe). L'identité des binaires de performance reste distincte dans les reçus. Les tests -race du reviewer compilent le candidat courant. Un premier lancement de tests a perdu son identifiant de session dans mon harnais ; sa sortie n'est pas comptée comme preuve. Le second lancement a corrigé la conservation des sorties, et fournit les originaux complets ci-dessous. Les petites erreurs de lecture de chemins/types du reviewer ont été corrigées par découverte/inspection ; elles ne sont pas attribuées au candidat.

## Reprise requise

Propriétaire des deux corrections : implémenteur/root. Le candidat est libéré à la fin de cette revue pour correction autorisée. Après corrections : rouge local puis vert de chaque frontière, documentation pertinente, nouveau SHA/identité binaire et revue renouvelée sur les changements ; réutiliser les contrôles critiques inchangés. Le refus de capacité appartient au fournisseur ; sa condition de reprise reste une disponibilité réellement vérifiée ou une décision explicite sur la politique, sans fallback silencieux. Les autres travaux indépendants peuvent continuer. Le verdict request changes bloque la clôture T7 et le RETEX final ; il ne clôt ni ne relance la mission historique.

## Originaux externes examinés (empreintes)

- `/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-provider-structured-original.json` : SHA-256 `6e1530e4cd480779d6c95beffb153260fb37c0507630ee982df035ac1c92f0d9`.
- `/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-provider-filtered-env-original.json` : SHA-256 `f80bcea09fe50c8b476f3f3d8778be0dda48a8d8d663ae35345f2b58406d935b`.
- `/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-original-planning-schema.json` : SHA-256 `37bbea9152b2f7910c170adf3127a19f8ee39f3e860356e91561e0a151b341e6`.
- `/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-real-codex-44a86e6/frozen.json` : SHA-256 `c28d0c0afb4299e4a38cf1fdd942278c8d8049323bf92af6cbad744dbedd2c66`.
- `/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-real-codex-44a86e6/current.json` : SHA-256 `391f524be24e62048513b4310b33c49204b33c255e0de5f4d93332932c98e85d`.

Rapport finalisé 2026-10-13T07:11:34.670933+00:00.

# Annexes : contrôles et reproducer conservés

## Reproducer original — historique navigateur plein

```json
{
  "candidate": "44a86e646c37da503a38e4a687aeb1a50ecb19e1",
  "at": "2026-10-13T07:06:17.949709+00:00",
  "binary": "/home/fpizzi/.cache/swarm-operator-replan-20261011/swarm-t7-44a86e6",
  "binary_sha256": "44dffab8564d0e9c75ab4354b381bafae155836f6aff53261ac7801a3075c73d",
  "fixture_root": "/tmp/swarm-t7-review-web-history-nfo39t5w",
  "public_successful_revisions": 418,
  "preview_exit": 0,
  "preview_proposed_revision": 419,
  "apply_exit": 2,
  "apply_stdout": "",
  "apply_stderr": "{\n  \"error\": \"Historique plein / History full\",\n  \"failure\": {\n    \"code\": \"command_failed\",\n    \"message\": \"Historique plein / History full\",\n    \"retryable\": false\n  },\n  \"schema_version\": 1\n}\n",
  "request": {
    "schema_version": 1,
    "event_id": "history-418",
    "expected_revision": 418,
    "values": {
      "general_ms": 1000,
      "snapshot_ms": 1000,
      "connection_test_ms": 1000
    },
    "reason": "rrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr"
  },
  "show_after_exit": 0
}
```

## Contrôles ciblés du reviewer

```json
{
  "command": [
    "go",
    "test",
    "-race",
    "./internal/engine",
    "-run",
    "^(TestPlanningJudge.*|TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending|TestCursorEngineT3ConcurrentScopesAndSharedReservations|TestCursorEngineT3Exploration.*|TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask|TestManagedPublicationLaneRetainsSiblingWithoutPaidReview|TestManagedPublicationStagingPreservesPauseAndIdentity|TestManagedRootAlias.*|TestGraphPerformance.*)$",
    "-count=1",
    "-timeout=180s",
    "-json"
  ],
  "started": "2026-10-13T07:06:03.447098+00:00",
  "finished": "2026-10-13T07:06:53.533024+00:00",
  "exit": 0,
  "top_level_passed": 32,
  "passed_including_subcases": 68,
  "failed": [],
  "passed_names": [
    "TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete",
    "TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause",
    "TestCursorEngineT3ConcurrentScopesAndSharedReservations",
    "TestCursorEngineT3ExplorationReadAndPersistentDiscovery",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/missing",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/generation",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/scope",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests/shell",
    "TestCursorEngineT3ExplorationRejectsInvalidRequests",
    "TestCursorEngineT3ExplorationInventoryAndSchemaAvailability",
    "TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask",
    "TestGraphPerformancePersistencePreviewReplayAndConflict",
    "TestGraphPerformanceConcurrentInvalidAndSymlink",
    "TestGraphPerformanceCLIHTTPParityAuthAndPreview",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles",
    "TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings",
    "TestGraphPerformanceRepresentationAndTransportBounds",
    "TestGraphPerformanceOversizePreviewAndApplyAgree",
    "TestManagedPublicationLaneRetainsSiblingWithoutPaidReview",
    "TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause",
    "TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause",
    "TestManagedPublicationStagingPreservesPauseAndIdentity/active",
    "TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt",
    "TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy",
    "TestManagedPublicationStagingPreservesPauseAndIdentity",
    "TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle",
    "TestManagedRootAliasPreparedCopyIsReused",
    "TestManagedRootAliasStillRejectsInnerRedirectedCopy",
    "TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy",
    "TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias",
    "TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace",
    "TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata",
    "TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata",
    "TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal",
    "TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child",
    "TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect",
    "TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end",
    "TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process",
    "TestPlanningJudgeStructuredEndRequiresNoActiveProcess",
    "TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext",
    "TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs",
    "TestPlanningJudgeVerdictDistinctFromCandidateReview",
    "TestPlanningJudgeEndRequiresFreshMatchingCandidate",
    "TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review",
    "TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control",
    "TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active",
    "TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess",
    "TestPlanningJudgeRenewalPreservesContextAndHistory",
    "TestPlanningJudgeReplayStaleAndDoubleVerdictRefused",
    "TestPlanningJudgePausedAndQuotaRefuseEffects",
    "TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending"
  ]
}
```

Sortie originale :

```jsonl
{"Time":"2026-10-13T09:06:07.130767889+02:00","Action":"start","Package":"swarm.local/companion/internal/engine"}
{"Time":"2026-10-13T09:06:07.376349458+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations"}
{"Time":"2026-10-13T09:06:07.376684858+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations","Output":"=== RUN   TestCursorEngineT3ConcurrentScopesAndSharedReservations\n"}
{"Time":"2026-10-13T09:06:07.376744871+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete"}
{"Time":"2026-10-13T09:06:07.376752516+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete","Output":"=== RUN   TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete\n"}
{"Time":"2026-10-13T09:06:08.309056207+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete","Output":"--- PASS: TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete (0.93s)\n"}
{"Time":"2026-10-13T09:06:08.309213784+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/complete","Elapsed":0.93}
{"Time":"2026-10-13T09:06:08.309303222+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause"}
{"Time":"2026-10-13T09:06:08.309314273+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause","Output":"=== RUN   TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause\n"}
{"Time":"2026-10-13T09:06:09.246869179+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause","Output":"--- PASS: TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause (0.94s)\n"}
{"Time":"2026-10-13T09:06:09.246917509+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations/pause","Elapsed":0.94}
{"Time":"2026-10-13T09:06:09.246932357+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations","Output":"--- PASS: TestCursorEngineT3ConcurrentScopesAndSharedReservations (1.87s)\n"}
{"Time":"2026-10-13T09:06:09.24693894+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ConcurrentScopesAndSharedReservations","Elapsed":1.87}
{"Time":"2026-10-13T09:06:09.246946213+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationReadAndPersistentDiscovery"}
{"Time":"2026-10-13T09:06:09.24697641+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationReadAndPersistentDiscovery","Output":"=== RUN   TestCursorEngineT3ExplorationReadAndPersistentDiscovery\n"}
{"Time":"2026-10-13T09:06:10.047249188+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationReadAndPersistentDiscovery","Output":"--- PASS: TestCursorEngineT3ExplorationReadAndPersistentDiscovery (0.80s)\n"}
{"Time":"2026-10-13T09:06:10.047301697+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationReadAndPersistentDiscovery","Elapsed":0.8}
{"Time":"2026-10-13T09:06:10.047405121+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests"}
{"Time":"2026-10-13T09:06:10.047420019+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests\n"}
{"Time":"2026-10-13T09:06:10.047550685+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute"}
{"Time":"2026-10-13T09:06:10.047559231+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute\n"}
{"Time":"2026-10-13T09:06:10.642215039+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute (0.59s)\n"}
{"Time":"2026-10-13T09:06:10.642298506+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/absolute","Elapsed":0.59}
{"Time":"2026-10-13T09:06:10.642313283+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal"}
{"Time":"2026-10-13T09:06:10.642319605+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal\n"}
{"Time":"2026-10-13T09:06:11.260493301+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal (0.62s)\n"}
{"Time":"2026-10-13T09:06:11.26057793+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/traversal","Elapsed":0.62}
{"Time":"2026-10-13T09:06:11.261458957+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate"}
{"Time":"2026-10-13T09:06:11.261483713+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate\n"}
{"Time":"2026-10-13T09:06:12.192528693+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate (0.93s)\n"}
{"Time":"2026-10-13T09:06:12.192877028+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/duplicate","Elapsed":0.93}
{"Time":"2026-10-13T09:06:12.193848124+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/missing"}
{"Time":"2026-10-13T09:06:12.193901544+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/missing","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/missing\n"}
{"Time":"2026-10-13T09:06:12.723942424+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/missing","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/missing (0.53s)\n"}
{"Time":"2026-10-13T09:06:12.724436513+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/missing","Elapsed":0.53}
{"Time":"2026-10-13T09:06:12.724496125+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate"}
{"Time":"2026-10-13T09:06:12.724527233+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate\n"}
{"Time":"2026-10-13T09:06:13.26792418+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate (0.54s)\n"}
{"Time":"2026-10-13T09:06:13.269074543+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/candidate","Elapsed":0.54}
{"Time":"2026-10-13T09:06:13.269129677+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/generation"}
{"Time":"2026-10-13T09:06:13.269231057+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/generation","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/generation\n"}
{"Time":"2026-10-13T09:06:13.937106942+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/generation","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/generation (0.67s)\n"}
{"Time":"2026-10-13T09:06:13.937157016+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/generation","Elapsed":0.67}
{"Time":"2026-10-13T09:06:13.937168508+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/scope"}
{"Time":"2026-10-13T09:06:13.937174419+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/scope","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/scope\n"}
{"Time":"2026-10-13T09:06:14.54104925+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/scope","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/scope (0.60s)\n"}
{"Time":"2026-10-13T09:06:14.5411075+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/scope","Elapsed":0.6}
{"Time":"2026-10-13T09:06:14.541117929+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field"}
{"Time":"2026-10-13T09:06:14.541122177+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field\n"}
{"Time":"2026-10-13T09:06:15.060913579+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field (0.52s)\n"}
{"Time":"2026-10-13T09:06:15.062000824+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/unknown-field","Elapsed":0.52}
{"Time":"2026-10-13T09:06:15.062033055+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/shell"}
{"Time":"2026-10-13T09:06:15.062039427+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/shell","Output":"=== RUN   TestCursorEngineT3ExplorationRejectsInvalidRequests/shell\n"}
{"Time":"2026-10-13T09:06:15.743699913+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/shell","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests/shell (0.68s)\n"}
{"Time":"2026-10-13T09:06:15.743865033+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests/shell","Elapsed":0.68}
{"Time":"2026-10-13T09:06:15.743904968+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests","Output":"--- PASS: TestCursorEngineT3ExplorationRejectsInvalidRequests (5.70s)\n"}
{"Time":"2026-10-13T09:06:15.743934764+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationRejectsInvalidRequests","Elapsed":5.7}
{"Time":"2026-10-13T09:06:15.743991171+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationInventoryAndSchemaAvailability"}
{"Time":"2026-10-13T09:06:15.744018071+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationInventoryAndSchemaAvailability","Output":"=== RUN   TestCursorEngineT3ExplorationInventoryAndSchemaAvailability\n"}
{"Time":"2026-10-13T09:06:16.518164928+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationInventoryAndSchemaAvailability","Output":"--- PASS: TestCursorEngineT3ExplorationInventoryAndSchemaAvailability (0.77s)\n"}
{"Time":"2026-10-13T09:06:16.518221705+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ExplorationInventoryAndSchemaAvailability","Elapsed":0.77}
{"Time":"2026-10-13T09:06:16.518327013+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask"}
{"Time":"2026-10-13T09:06:16.518336381+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask","Output":"=== RUN   TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask\n"}
{"Time":"2026-10-13T09:06:18.067582704+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask","Output":"--- PASS: TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask (1.55s)\n"}
{"Time":"2026-10-13T09:06:18.06763869+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask","Elapsed":1.55}
{"Time":"2026-10-13T09:06:18.06866498+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformancePersistencePreviewReplayAndConflict"}
{"Time":"2026-10-13T09:06:18.068694205+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformancePersistencePreviewReplayAndConflict","Output":"=== RUN   TestGraphPerformancePersistencePreviewReplayAndConflict\n"}
{"Time":"2026-10-13T09:06:18.425947392+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformancePersistencePreviewReplayAndConflict","Output":"--- PASS: TestGraphPerformancePersistencePreviewReplayAndConflict (0.36s)\n"}
{"Time":"2026-10-13T09:06:18.426181822+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformancePersistencePreviewReplayAndConflict","Elapsed":0.36}
{"Time":"2026-10-13T09:06:18.426214835+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceConcurrentInvalidAndSymlink"}
{"Time":"2026-10-13T09:06:18.426224352+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceConcurrentInvalidAndSymlink","Output":"=== RUN   TestGraphPerformanceConcurrentInvalidAndSymlink\n"}
{"Time":"2026-10-13T09:06:18.756120191+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceConcurrentInvalidAndSymlink","Output":"--- PASS: TestGraphPerformanceConcurrentInvalidAndSymlink (0.33s)\n"}
{"Time":"2026-10-13T09:06:18.756152652+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceConcurrentInvalidAndSymlink","Elapsed":0.33}
{"Time":"2026-10-13T09:06:18.756278649+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceCLIHTTPParityAuthAndPreview"}
{"Time":"2026-10-13T09:06:18.75628993+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceCLIHTTPParityAuthAndPreview","Output":"=== RUN   TestGraphPerformanceCLIHTTPParityAuthAndPreview\n"}
{"Time":"2026-10-13T09:06:19.082088021+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceCLIHTTPParityAuthAndPreview","Output":"--- PASS: TestGraphPerformanceCLIHTTPParityAuthAndPreview (0.33s)\n"}
{"Time":"2026-10-13T09:06:19.082147884+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceCLIHTTPParityAuthAndPreview","Elapsed":0.33}
{"Time":"2026-10-13T09:06:19.082349393+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings"}
{"Time":"2026-10-13T09:06:19.082360524+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings\n"}
{"Time":"2026-10-13T09:06:19.082368118+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard"}
{"Time":"2026-10-13T09:06:19.082371835+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard\n"}
{"Time":"2026-10-13T09:06:19.510188464+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard (0.43s)\n"}
{"Time":"2026-10-13T09:06:19.510252996+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/keyboard","Elapsed":0.43}
{"Time":"2026-10-13T09:06:19.510448674+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples"}
{"Time":"2026-10-13T09:06:19.510462389+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples\n"}
{"Time":"2026-10-13T09:06:19.910895461+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples (0.40s)\n"}
{"Time":"2026-10-13T09:06:19.910936107+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/samples","Elapsed":0.4}
{"Time":"2026-10-13T09:06:19.910983356+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards"}
{"Time":"2026-10-13T09:06:19.910988616+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards\n"}
{"Time":"2026-10-13T09:06:20.26281182+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards (0.35s)\n"}
{"Time":"2026-10-13T09:06:20.262868066+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/cards","Elapsed":0.35}
{"Time":"2026-10-13T09:06:20.262879878+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render"}
{"Time":"2026-10-13T09:06:20.262884897+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render\n"}
{"Time":"2026-10-13T09:06:20.568158219+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render (0.30s)\n"}
{"Time":"2026-10-13T09:06:20.568214255+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/render","Elapsed":0.3}
{"Time":"2026-10-13T09:06:20.568228331+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles"}
{"Time":"2026-10-13T09:06:20.568237218+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles","Output":"=== RUN   TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles\n"}
{"Time":"2026-10-13T09:06:20.921690317+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles (0.35s)\n"}
{"Time":"2026-10-13T09:06:20.921755259+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings/profiles","Elapsed":0.35}
{"Time":"2026-10-13T09:06:20.921779615+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings","Output":"--- PASS: TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings (1.84s)\n"}
{"Time":"2026-10-13T09:06:20.921792509+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOperatorTargetsWithoutHistoricalCeilings","Elapsed":1.84}
{"Time":"2026-10-13T09:06:20.921985021+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceRepresentationAndTransportBounds"}
{"Time":"2026-10-13T09:06:20.922045084+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceRepresentationAndTransportBounds","Output":"=== RUN   TestGraphPerformanceRepresentationAndTransportBounds\n"}
{"Time":"2026-10-13T09:06:21.327307326+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceRepresentationAndTransportBounds","Output":"--- PASS: TestGraphPerformanceRepresentationAndTransportBounds (0.41s)\n"}
{"Time":"2026-10-13T09:06:21.327358462+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceRepresentationAndTransportBounds","Elapsed":0.41}
{"Time":"2026-10-13T09:06:21.327544642+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOversizePreviewAndApplyAgree"}
{"Time":"2026-10-13T09:06:21.327558569+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOversizePreviewAndApplyAgree","Output":"=== RUN   TestGraphPerformanceOversizePreviewAndApplyAgree\n"}
{"Time":"2026-10-13T09:06:22.865114046+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOversizePreviewAndApplyAgree","Output":"--- PASS: TestGraphPerformanceOversizePreviewAndApplyAgree (1.54s)\n"}
{"Time":"2026-10-13T09:06:22.867012416+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphPerformanceOversizePreviewAndApplyAgree","Elapsed":1.54}
{"Time":"2026-10-13T09:06:22.867110911+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationLaneRetainsSiblingWithoutPaidReview"}
{"Time":"2026-10-13T09:06:22.867119126+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationLaneRetainsSiblingWithoutPaidReview","Output":"=== RUN   TestManagedPublicationLaneRetainsSiblingWithoutPaidReview\n"}
{"Time":"2026-10-13T09:06:24.858282489+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationLaneRetainsSiblingWithoutPaidReview","Output":"--- PASS: TestManagedPublicationLaneRetainsSiblingWithoutPaidReview (1.99s)\n"}
{"Time":"2026-10-13T09:06:24.858351259+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationLaneRetainsSiblingWithoutPaidReview","Elapsed":1.99}
{"Time":"2026-10-13T09:06:24.858372619+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity"}
{"Time":"2026-10-13T09:06:24.858381726+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity\n"}
{"Time":"2026-10-13T09:06:24.858897165+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause"}
{"Time":"2026-10-13T09:06:24.858911933+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause\n"}
{"Time":"2026-10-13T09:06:25.50530736+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause (0.65s)\n"}
{"Time":"2026-10-13T09:06:25.505364537+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/global-pause","Elapsed":0.65}
{"Time":"2026-10-13T09:06:25.505455088+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause"}
{"Time":"2026-10-13T09:06:25.505462712+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause\n"}
{"Time":"2026-10-13T09:06:26.14399949+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause (0.64s)\n"}
{"Time":"2026-10-13T09:06:26.14405269+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/planning-pause","Elapsed":0.64}
{"Time":"2026-10-13T09:06:26.144065073+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/active"}
{"Time":"2026-10-13T09:06:26.144071034+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/active","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity/active\n"}
{"Time":"2026-10-13T09:06:26.752552179+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/active","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity/active (0.61s)\n"}
{"Time":"2026-10-13T09:06:26.752609897+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/active","Elapsed":0.61}
{"Time":"2026-10-13T09:06:26.75268576+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt"}
{"Time":"2026-10-13T09:06:26.752694276+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt\n"}
{"Time":"2026-10-13T09:06:27.410056325+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt (0.66s)\n"}
{"Time":"2026-10-13T09:06:27.410127499+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-attempt","Elapsed":0.66}
{"Time":"2026-10-13T09:06:27.410307758+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy"}
{"Time":"2026-10-13T09:06:27.410321774+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy","Output":"=== RUN   TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy\n"}
{"Time":"2026-10-13T09:06:28.137756342+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy (0.73s)\n"}
{"Time":"2026-10-13T09:06:28.137812789+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity/foreign-copy","Elapsed":0.73}
{"Time":"2026-10-13T09:06:28.137853145+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity","Output":"--- PASS: TestManagedPublicationStagingPreservesPauseAndIdentity (3.28s)\n"}
{"Time":"2026-10-13T09:06:28.137913829+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedPublicationStagingPreservesPauseAndIdentity","Elapsed":3.28}
{"Time":"2026-10-13T09:06:28.140127432+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle"}
{"Time":"2026-10-13T09:06:28.140156306+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle","Output":"=== RUN   TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle\n"}
{"Time":"2026-10-13T09:06:28.808132005+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle","Output":"--- PASS: TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle (0.67s)\n"}
{"Time":"2026-10-13T09:06:28.808168774+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle","Elapsed":0.67}
{"Time":"2026-10-13T09:06:28.808261849+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPreparedCopyIsReused"}
{"Time":"2026-10-13T09:06:28.808268481+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPreparedCopyIsReused","Output":"=== RUN   TestManagedRootAliasPreparedCopyIsReused\n"}
{"Time":"2026-10-13T09:06:29.417625721+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPreparedCopyIsReused","Output":"--- PASS: TestManagedRootAliasPreparedCopyIsReused (0.61s)\n"}
{"Time":"2026-10-13T09:06:29.417674142+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPreparedCopyIsReused","Elapsed":0.61}
{"Time":"2026-10-13T09:06:29.417695943+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStillRejectsInnerRedirectedCopy"}
{"Time":"2026-10-13T09:06:29.417705351+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStillRejectsInnerRedirectedCopy","Output":"=== RUN   TestManagedRootAliasStillRejectsInnerRedirectedCopy\n"}
{"Time":"2026-10-13T09:06:30.091535859+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStillRejectsInnerRedirectedCopy","Output":"--- PASS: TestManagedRootAliasStillRejectsInnerRedirectedCopy (0.67s)\n"}
{"Time":"2026-10-13T09:06:30.091584481+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStillRejectsInnerRedirectedCopy","Elapsed":0.67}
{"Time":"2026-10-13T09:06:30.09160518+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy"}
{"Time":"2026-10-13T09:06:30.091611792+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy","Output":"=== RUN   TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy\n"}
{"Time":"2026-10-13T09:06:31.357936874+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy","Output":"--- PASS: TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy (1.27s)\n"}
{"Time":"2026-10-13T09:06:31.358247097+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy","Elapsed":1.27}
{"Time":"2026-10-13T09:06:31.358262987+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias"}
{"Time":"2026-10-13T09:06:31.358269299+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias","Output":"=== RUN   TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias\n"}
{"Time":"2026-10-13T09:06:32.149546385+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias","Output":"--- PASS: TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias (0.79s)\n"}
{"Time":"2026-10-13T09:06:32.149601188+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias","Elapsed":0.79}
{"Time":"2026-10-13T09:06:32.149659207+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata"}
{"Time":"2026-10-13T09:06:32.149663946+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata","Output":"=== RUN   TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata\n"}
{"Time":"2026-10-13T09:06:32.149740099+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace"}
{"Time":"2026-10-13T09:06:32.149744788+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace","Output":"=== RUN   TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace\n"}
{"Time":"2026-10-13T09:06:32.732536546+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace","Output":"--- PASS: TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace (0.58s)\n"}
{"Time":"2026-10-13T09:06:32.732593252+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/input-workspace","Elapsed":0.58}
{"Time":"2026-10-13T09:06:32.732690285+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata"}
{"Time":"2026-10-13T09:06:32.732697739+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata","Output":"=== RUN   TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata\n"}
{"Time":"2026-10-13T09:06:33.367796504+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata","Output":"--- PASS: TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata (0.63s)\n"}
{"Time":"2026-10-13T09:06:33.367851918+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata/git-metadata","Elapsed":0.63}
{"Time":"2026-10-13T09:06:33.367880953+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata","Output":"--- PASS: TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata (1.22s)\n"}
{"Time":"2026-10-13T09:06:33.368091088+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata","Elapsed":1.22}
{"Time":"2026-10-13T09:06:33.369487865+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal"}
{"Time":"2026-10-13T09:06:33.369500679+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal","Output":"=== RUN   TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal\n"}
{"Time":"2026-10-13T09:06:34.068998432+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal","Output":"--- PASS: TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal (0.70s)\n"}
{"Time":"2026-10-13T09:06:34.0690609+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal","Elapsed":0.7}
{"Time":"2026-10-13T09:06:34.069110162+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle"}
{"Time":"2026-10-13T09:06:34.069117085+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle","Output":"=== RUN   TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle\n"}
{"Time":"2026-10-13T09:06:34.478522211+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle","Output":"--- PASS: TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle (0.41s)\n"}
{"Time":"2026-10-13T09:06:34.478580591+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle","Elapsed":0.41}
{"Time":"2026-10-13T09:06:34.478597583+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect"}
{"Time":"2026-10-13T09:06:34.478604185+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect\n"}
{"Time":"2026-10-13T09:06:34.478610146+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle"}
{"Time":"2026-10-13T09:06:34.478615416+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle\n"}
{"Time":"2026-10-13T09:06:34.866038592+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle (0.39s)\n"}
{"Time":"2026-10-13T09:06:34.866094167+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/stale-cycle","Elapsed":0.39}
{"Time":"2026-10-13T09:06:34.866107262+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations"}
{"Time":"2026-10-13T09:06:34.866112371+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations\n"}
{"Time":"2026-10-13T09:06:35.35350543+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations (0.49s)\n"}
{"Time":"2026-10-13T09:06:35.353556927+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/mixed-operations","Elapsed":0.49}
{"Time":"2026-10-13T09:06:35.353576644+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields"}
{"Time":"2026-10-13T09:06:35.353585261+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields\n"}
{"Time":"2026-10-13T09:06:35.858520489+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields (0.50s)\n"}
{"Time":"2026-10-13T09:06:35.858576374+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/foreign-fields","Elapsed":0.5}
{"Time":"2026-10-13T09:06:35.859013917+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs"}
{"Time":"2026-10-13T09:06:35.859043953+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs\n"}
{"Time":"2026-10-13T09:06:36.225175368+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs (0.37s)\n"}
{"Time":"2026-10-13T09:06:36.225221064+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/end-without-proofs","Elapsed":0.37}
{"Time":"2026-10-13T09:06:36.22526713+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict"}
{"Time":"2026-10-13T09:06:36.225273633+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict\n"}
{"Time":"2026-10-13T09:06:36.608769242+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict (0.38s)\n"}
{"Time":"2026-10-13T09:06:36.608819406+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/renew-without-verdict","Elapsed":0.38}
{"Time":"2026-10-13T09:06:36.608840506+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task"}
{"Time":"2026-10-13T09:06:36.608850144+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task\n"}
{"Time":"2026-10-13T09:06:36.967627939+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task (0.36s)\n"}
{"Time":"2026-10-13T09:06:36.967681289+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/cycle-fields-on-task","Elapsed":0.36}
{"Time":"2026-10-13T09:06:36.96770823+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause"}
{"Time":"2026-10-13T09:06:36.967717407+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause\n"}
{"Time":"2026-10-13T09:06:37.402772027+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause (0.44s)\n"}
{"Time":"2026-10-13T09:06:37.402840125+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/global-pause","Elapsed":0.44}
{"Time":"2026-10-13T09:06:37.403250056+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child"}
{"Time":"2026-10-13T09:06:37.403295852+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child","Output":"=== RUN   TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child\n"}
{"Time":"2026-10-13T09:06:37.899594668+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child (0.50s)\n"}
{"Time":"2026-10-13T09:06:37.899639833+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect/child","Elapsed":0.5}
{"Time":"2026-10-13T09:06:37.899680369+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect","Output":"--- PASS: TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect (3.42s)\n"}
{"Time":"2026-10-13T09:06:37.899688855+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect","Elapsed":3.42}
{"Time":"2026-10-13T09:06:37.899828678+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess"}
{"Time":"2026-10-13T09:06:37.899857332+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess","Output":"=== RUN   TestPlanningJudgeStructuredEndRequiresNoActiveProcess\n"}
{"Time":"2026-10-13T09:06:37.899880175+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end"}
{"Time":"2026-10-13T09:06:37.899897507+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end","Output":"=== RUN   TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end\n"}
{"Time":"2026-10-13T09:06:39.953396226+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end","Output":"--- PASS: TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end (2.05s)\n"}
{"Time":"2026-10-13T09:06:39.953453163+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/fresh-end","Elapsed":2.05}
{"Time":"2026-10-13T09:06:39.953470275+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process"}
{"Time":"2026-10-13T09:06:39.953476667+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process","Output":"=== RUN   TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process\n"}
{"Time":"2026-10-13T09:06:41.749430063+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process","Output":"--- PASS: TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process (1.80s)\n"}
{"Time":"2026-10-13T09:06:41.751251158+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess/active-process","Elapsed":1.8}
{"Time":"2026-10-13T09:06:41.751313355+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess","Output":"--- PASS: TestPlanningJudgeStructuredEndRequiresNoActiveProcess (3.85s)\n"}
{"Time":"2026-10-13T09:06:41.751332692+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeStructuredEndRequiresNoActiveProcess","Elapsed":3.85}
{"Time":"2026-10-13T09:06:41.751339785+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext"}
{"Time":"2026-10-13T09:06:41.751345095+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext","Output":"=== RUN   TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext\n"}
{"Time":"2026-10-13T09:06:42.458665812+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext","Output":"--- PASS: TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext (0.71s)\n"}
{"Time":"2026-10-13T09:06:42.458708332+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext","Elapsed":0.71}
{"Time":"2026-10-13T09:06:42.458717539+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs"}
{"Time":"2026-10-13T09:06:42.458721958+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs","Output":"=== RUN   TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs\n"}
{"Time":"2026-10-13T09:06:42.911185884+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs","Output":"--- PASS: TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs (0.45s)\n"}
{"Time":"2026-10-13T09:06:42.911248522+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs","Elapsed":0.45}
{"Time":"2026-10-13T09:06:42.911424112+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeVerdictDistinctFromCandidateReview"}
{"Time":"2026-10-13T09:06:42.911435524+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeVerdictDistinctFromCandidateReview","Output":"=== RUN   TestPlanningJudgeVerdictDistinctFromCandidateReview\n"}
{"Time":"2026-10-13T09:06:43.353914182+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeVerdictDistinctFromCandidateReview","Output":"--- PASS: TestPlanningJudgeVerdictDistinctFromCandidateReview (0.44s)\n"}
{"Time":"2026-10-13T09:06:43.354001436+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeVerdictDistinctFromCandidateReview","Elapsed":0.44}
{"Time":"2026-10-13T09:06:43.354019119+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRequiresFreshMatchingCandidate"}
{"Time":"2026-10-13T09:06:43.354029459+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRequiresFreshMatchingCandidate","Output":"=== RUN   TestPlanningJudgeEndRequiresFreshMatchingCandidate\n"}
{"Time":"2026-10-13T09:06:45.419811261+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRequiresFreshMatchingCandidate","Output":"--- PASS: TestPlanningJudgeEndRequiresFreshMatchingCandidate (2.07s)\n"}
{"Time":"2026-10-13T09:06:45.419874049+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRequiresFreshMatchingCandidate","Elapsed":2.07}
{"Time":"2026-10-13T09:06:45.420116315+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess"}
{"Time":"2026-10-13T09:06:45.420133527+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess","Output":"=== RUN   TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess\n"}
{"Time":"2026-10-13T09:06:45.420269092+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review"}
{"Time":"2026-10-13T09:06:45.42028388+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review","Output":"=== RUN   TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review\n"}
{"Time":"2026-10-13T09:06:47.161892563+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review","Output":"--- PASS: TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review (1.74s)\n"}
{"Time":"2026-10-13T09:06:47.164056303+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/review","Elapsed":1.74}
{"Time":"2026-10-13T09:06:47.164109963+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control"}
{"Time":"2026-10-13T09:06:47.164116906+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control","Output":"=== RUN   TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control\n"}
{"Time":"2026-10-13T09:06:48.84577802+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control","Output":"--- PASS: TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control (1.68s)\n"}
{"Time":"2026-10-13T09:06:48.845833164+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/control","Elapsed":1.68}
{"Time":"2026-10-13T09:06:48.845901612+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active"}
{"Time":"2026-10-13T09:06:48.845908706+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active","Output":"=== RUN   TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active\n"}
{"Time":"2026-10-13T09:06:50.76494163+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active","Output":"--- PASS: TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active (1.92s)\n"}
{"Time":"2026-10-13T09:06:50.765002916+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess/active","Elapsed":1.92}
{"Time":"2026-10-13T09:06:50.765016762+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess","Output":"--- PASS: TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess (5.34s)\n"}
{"Time":"2026-10-13T09:06:50.76503726+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess","Elapsed":5.34}
{"Time":"2026-10-13T09:06:50.765183816+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeRenewalPreservesContextAndHistory"}
{"Time":"2026-10-13T09:06:50.765214784+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeRenewalPreservesContextAndHistory","Output":"=== RUN   TestPlanningJudgeRenewalPreservesContextAndHistory\n"}
{"Time":"2026-10-13T09:06:51.194307595+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeRenewalPreservesContextAndHistory","Output":"--- PASS: TestPlanningJudgeRenewalPreservesContextAndHistory (0.43s)\n"}
{"Time":"2026-10-13T09:06:51.19435822+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeRenewalPreservesContextAndHistory","Elapsed":0.43}
{"Time":"2026-10-13T09:06:51.194370804+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeReplayStaleAndDoubleVerdictRefused"}
{"Time":"2026-10-13T09:06:51.194376154+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeReplayStaleAndDoubleVerdictRefused","Output":"=== RUN   TestPlanningJudgeReplayStaleAndDoubleVerdictRefused\n"}
{"Time":"2026-10-13T09:06:51.641382802+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeReplayStaleAndDoubleVerdictRefused","Output":"--- PASS: TestPlanningJudgeReplayStaleAndDoubleVerdictRefused (0.45s)\n"}
{"Time":"2026-10-13T09:06:51.641427897+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgeReplayStaleAndDoubleVerdictRefused","Elapsed":0.45}
{"Time":"2026-10-13T09:06:51.641443586+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgePausedAndQuotaRefuseEffects"}
{"Time":"2026-10-13T09:06:51.641449227+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgePausedAndQuotaRefuseEffects","Output":"=== RUN   TestPlanningJudgePausedAndQuotaRefuseEffects\n"}
{"Time":"2026-10-13T09:06:51.997773481+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgePausedAndQuotaRefuseEffects","Output":"--- PASS: TestPlanningJudgePausedAndQuotaRefuseEffects (0.36s)\n"}
{"Time":"2026-10-13T09:06:51.997821982+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningJudgePausedAndQuotaRefuseEffects","Elapsed":0.36}
{"Time":"2026-10-13T09:06:51.998030113+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending"}
{"Time":"2026-10-13T09:06:51.998048999+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending","Output":"=== RUN   TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending\n"}
{"Time":"2026-10-13T09:06:52.450620572+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending","Output":"--- PASS: TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending (0.45s)\n"}
{"Time":"2026-10-13T09:06:52.450669023+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending","Elapsed":0.45}
{"Time":"2026-10-13T09:06:52.453816442+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"PASS\n"}
{"Time":"2026-10-13T09:06:53.476215034+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"ok  \tswarm.local/companion/internal/engine\t46.344s\n"}
{"Time":"2026-10-13T09:06:53.476258085+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Elapsed":46.345}

```

Stderr original :

```text

```

## Contrôles supplémentaires — contrats de politique et bail

```json
{
  "command": [
    "go",
    "test",
    "-race",
    "./internal/engine",
    "-run",
    "^(TestWebRequests.*|TestQuotas.*|TestConfiguredQuotasBeyondFormerStaticCeilings|TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation|TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner)$",
    "-count=1",
    "-timeout=90s",
    "-json"
  ],
  "started": "2026-10-13T07:08:14.005300+00:00",
  "finished": "2026-10-13T07:08:28.942487+00:00",
  "exit": 0,
  "top_level_passed": 9,
  "passed_including_subcases": 9,
  "failed": [],
  "passed_names": [
    "TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation",
    "TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner",
    "TestConfiguredQuotasBeyondFormerStaticCeilings",
    "TestQuotasPublicParityPersistenceAndNoRefund",
    "TestQuotasRejectActivePlannerAndInvalidBudgets",
    "TestQuotasConcurrentAuthorization",
    "TestWebRequestsPublicPersistencePreviewReplayAndConflict",
    "TestWebRequestsInvalidAuthenticationAndConcurrentRevision",
    "TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy"
  ]
}
```

Sortie originale :

```jsonl
{"Time":"2026-10-13T09:08:17.586504155+02:00","Action":"start","Package":"swarm.local/companion/internal/engine"}
{"Time":"2026-10-13T09:08:17.853470626+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation"}
{"Time":"2026-10-13T09:08:17.853603386+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation","Output":"=== RUN   TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation\n"}
{"Time":"2026-10-13T09:08:24.544040721+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation","Output":"--- PASS: TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation (6.69s)\n"}
{"Time":"2026-10-13T09:08:24.544855863+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation","Elapsed":6.69}
{"Time":"2026-10-13T09:08:24.544886892+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner"}
{"Time":"2026-10-13T09:08:24.544893755+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner","Output":"=== RUN   TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner\n"}
{"Time":"2026-10-13T09:08:24.890768057+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner","Output":"--- PASS: TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner (0.35s)\n"}
{"Time":"2026-10-13T09:08:24.890817991+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner","Elapsed":0.35}
{"Time":"2026-10-13T09:08:24.890839071+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestConfiguredQuotasBeyondFormerStaticCeilings"}
{"Time":"2026-10-13T09:08:24.890844401+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestConfiguredQuotasBeyondFormerStaticCeilings","Output":"=== RUN   TestConfiguredQuotasBeyondFormerStaticCeilings\n"}
{"Time":"2026-10-13T09:08:25.453311766+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestConfiguredQuotasBeyondFormerStaticCeilings","Output":"--- PASS: TestConfiguredQuotasBeyondFormerStaticCeilings (0.56s)\n"}
{"Time":"2026-10-13T09:08:25.453360157+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestConfiguredQuotasBeyondFormerStaticCeilings","Elapsed":0.56}
{"Time":"2026-10-13T09:08:25.453421573+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasPublicParityPersistenceAndNoRefund"}
{"Time":"2026-10-13T09:08:25.453428406+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasPublicParityPersistenceAndNoRefund","Output":"=== RUN   TestQuotasPublicParityPersistenceAndNoRefund\n"}
{"Time":"2026-10-13T09:08:26.033442107+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasPublicParityPersistenceAndNoRefund","Output":"--- PASS: TestQuotasPublicParityPersistenceAndNoRefund (0.58s)\n"}
{"Time":"2026-10-13T09:08:26.033491039+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasPublicParityPersistenceAndNoRefund","Elapsed":0.58}
{"Time":"2026-10-13T09:08:26.033511978+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasRejectActivePlannerAndInvalidBudgets"}
{"Time":"2026-10-13T09:08:26.033520615+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasRejectActivePlannerAndInvalidBudgets","Output":"=== RUN   TestQuotasRejectActivePlannerAndInvalidBudgets\n"}
{"Time":"2026-10-13T09:08:26.375204366+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasRejectActivePlannerAndInvalidBudgets","Output":"--- PASS: TestQuotasRejectActivePlannerAndInvalidBudgets (0.34s)\n"}
{"Time":"2026-10-13T09:08:26.375258568+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasRejectActivePlannerAndInvalidBudgets","Elapsed":0.34}
{"Time":"2026-10-13T09:08:26.37527593+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasConcurrentAuthorization"}
{"Time":"2026-10-13T09:08:26.375284637+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasConcurrentAuthorization","Output":"=== RUN   TestQuotasConcurrentAuthorization\n"}
{"Time":"2026-10-13T09:08:26.752208496+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasConcurrentAuthorization","Output":"--- PASS: TestQuotasConcurrentAuthorization (0.38s)\n"}
{"Time":"2026-10-13T09:08:26.752252338+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestQuotasConcurrentAuthorization","Elapsed":0.38}
{"Time":"2026-10-13T09:08:26.753565227+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsPublicPersistencePreviewReplayAndConflict"}
{"Time":"2026-10-13T09:08:26.753587659+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsPublicPersistencePreviewReplayAndConflict","Output":"=== RUN   TestWebRequestsPublicPersistencePreviewReplayAndConflict\n"}
{"Time":"2026-10-13T09:08:27.111489939+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsPublicPersistencePreviewReplayAndConflict","Output":"--- PASS: TestWebRequestsPublicPersistencePreviewReplayAndConflict (0.36s)\n"}
{"Time":"2026-10-13T09:08:27.111539782+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsPublicPersistencePreviewReplayAndConflict","Elapsed":0.36}
{"Time":"2026-10-13T09:08:27.111561884+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsInvalidAuthenticationAndConcurrentRevision"}
{"Time":"2026-10-13T09:08:27.111571492+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsInvalidAuthenticationAndConcurrentRevision","Output":"=== RUN   TestWebRequestsInvalidAuthenticationAndConcurrentRevision\n"}
{"Time":"2026-10-13T09:08:27.488137417+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsInvalidAuthenticationAndConcurrentRevision","Output":"--- PASS: TestWebRequestsInvalidAuthenticationAndConcurrentRevision (0.38s)\n"}
{"Time":"2026-10-13T09:08:27.488188694+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsInvalidAuthenticationAndConcurrentRevision","Elapsed":0.38}
{"Time":"2026-10-13T09:08:27.488210735+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy"}
{"Time":"2026-10-13T09:08:27.488219972+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy","Output":"=== RUN   TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy\n"}
{"Time":"2026-10-13T09:08:27.862291277+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy","Output":"--- PASS: TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy (0.37s)\n"}
{"Time":"2026-10-13T09:08:27.862334728+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy","Elapsed":0.37}
{"Time":"2026-10-13T09:08:27.862563529+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"PASS\n"}
{"Time":"2026-10-13T09:08:28.876410832+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"ok  \tswarm.local/companion/internal/engine\t11.289s\n"}
{"Time":"2026-10-13T09:08:28.876462389+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Elapsed":11.29}

```

Stderr original :

```text

```
