# T2 — Suivi de reprise (tentative 3/3)

## Identité et scope

- Mission w-843bb3ce22cff2965c5e77b6 ; tâche plan-843bb3ce22-T2 ; rôle worker.
- Agent auto-0b1a66811cf645081a7a ; tentative a-0f8f021749fd51e0a2f3daf9 ;
  départ 3/3 (dernier départ disponible sur cette tâche) ; révision de travail 167.
- Racine de travail vérifiée : `/home/fpizzi/workspace/swarm-engine-contract/source`
  (`pwd` et `git rev-parse --show-toplevel` concordent). Branche
  `codex/engine-review-contract`. HEAD 097e745, arbre local avec modifications
  non committées préexistantes (nombreux fichiers `M`/`??`, non touchées par
  cette tentative).
- Livrable assigné : `docs/T2-contrat-moteur.md` (déjà présent, non modifié par
  cette tentative — voir constat sous « Limites »).

## Ce que cette tentative a fait

Aucun fichier applicatif n'a été modifié. Reprise : lecture du rapport existant
(`docs/plan-843bb3ce22-T2.md`, écrit le 2026-09-29 par le superviseur Codex après
une production initiale de l'agent Claude `auto-b20a4cd7f84cb5da2842`), puis
re-vérification complète et fraîche des quatre critères assignés sur l'état
actuel du code (les fichiers `run_limits.go`, `run_limits_admin.go` et
`agents_store.go` ont un horodatage du 2026-09-30 matin, donc modifiés par un
départ antérieur à celui-ci après la rédaction du rapport du 29 — retestés ici,
pas supposés inchangés).

## Preuves fraîches (commandes exécutées par cette tentative, ce jour)

| Contrôle | Commande | Résultat |
| --- | --- | --- |
| Compilation | `go build ./...` | Code 0, aucune sortie. |
| Vet | `go vet ./...` | Code 0, aucune sortie. |
| Tests ciblés RunLimits (moteur + CLI/web hors scope) | `go test ./... -run '^TestRunLimits' -v -count=1 -timeout 60s` | Code 0, 12/12 PASS (liste ci-dessous). |
| Câblage du critère 4 toujours présent | `rg -n "configuredRunLimitsWith|cappedBy\(adminLimits\)" agents_store.go` | Lignes 580/589 présentes, inchangées fonctionnellement. |

Journal complet des tests (`/tmp/t2_test_run.log`, ce jour) :

```text
--- PASS: TestRunLimitsConfigRejectsInvalidValue (0.07s)
--- PASS: TestRunLimitsConfigPersistsAfterRealRestart (0.10s)
--- PASS: TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected (0.14s)
--- PASS: TestRunLimitsConfigConcurrencyAndReplay (0.07s)
--- PASS: TestRunLimitsMigrationKeepsExistingConfiguration (0.13s)
--- PASS: TestRunLimitsCLIRefusesSameInvalidValuesAsEngine (0.08s)
--- PASS: TestRunLimitsCLIAppliesShowsHistoryAndRollsBack (0.10s)
--- PASS: TestRunLimitsActualLaunchFreezesConfiguration (0.20s)
--- PASS: TestRunLimitsLaunchCannotRaiseCeilings (0.19s)
--- PASS: TestRunLimitsAdminHTTPRoundTrip (0.07s)
--- PASS: TestRunLimitsAdminHTTPRejectsCrossSiteWrite (0.06s)
--- PASS: TestRunLimitsFieldBoundsMatchValidator (0.00s)
PASS
ok  	swarm.local/companion	1.266s
```

Les tests CLI/web (`TestRunLimitsCLI*`, `TestRunLimitsAdminHTTP*`) appartiennent
au périmètre T3/T4, pas à T2 ; ils sont rapportés ici uniquement comme preuve de
non-régression croisée, pas comme critère validé par cette tâche.

## Matrice des critères assignés (req-4 à req-7)

| ID | Critère | Preuve (ce jour) | Résultat |
| --- | --- | --- | --- |
| req-4 | Refus testé de valeur invalide | `TestRunLimitsConfigRejectsInvalidValue` : rejette `max_tool_calls` hors borne, `silence_seconds` négatif, portée inconnue, mission vide en portée mission ; accepte l'override tout-zéro (héritage pur). | PASS |
| req-5 | Persistance vérifiée après redémarrage réel | `TestRunLimitsConfigPersistsAfterRealRestart` : ferme réellement `s.db`, réouvre via `openStore`, relit révision 1 / 42 appels et la résolution effective post-redémarrage. | PASS |
| req-6 | Historique et rollback fonctionnels et testés | `TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected` : 3 révisions conservées en historique, rollback vers la valeur 30 avec `rollback_of=1` sur une **nouvelle** révision (3), rollback vers révision inexistante refusé. `TestRunLimitsConfigConcurrencyAndReplay` : conflit de révision périmée détecté, rejeu strictement identique idempotent, réutilisation d'`event_id` avec contenu différent refusée. | PASS |
| req-7 | Distinction futurs départs / tentatives en cours (REQ-ADM-05) | `TestRunLimitsActualLaunchFreezesConfiguration` : premier agent réellement réservé (`prepareLaunch`) à 30 appels via `Store.agent()` relu en base ; changement de la config rôle à 12 après coup ; premier agent toujours à 30 en relecture ; second agent (nouvelle tâche) réservé et relu à 12. `TestRunLimitsLaunchCannotRaiseCeilings` : un override admin à 900 ne relève ni le plafond fournisseur (100) ni un plafond explicite de la requête (7). | PASS |

Les 4 critères assignés sont vérifiés avec preuve d'exécution réelle (vraie base
SQLite temporaire, vrai `Store.prepareLaunch`, pas de double/mock du
comportement testé), fraîche à cette tentative.

## Constat non bloquant pour le responsable

`docs/T2-contrat-moteur.md` (le livrable) est **identique** (diff vide,
432 lignes) à `docs/plan-843bb3ce22-T2.md` tel qu'écrit par la tentative
précédente. Il documente les preuves de test et l'implémentation du critère 4,
mais ne contient pas de section « contrat » séparée (schéma de champs,
unités/bornes énumérées indépendamment du code, sémantique d'API) au-delà des
extraits de code cités. Le schéma réel (`RunLimits` dans `run_limits.go:9-16` :
`observation_mode`, `silence_seconds`, `tool_seconds`, `max_tool_calls`,
`max_repeated_calls`, `max_consecutive_errors` ; portées `project/mission/role/
task` dans `run_limits_admin.go:28-31`) ne couvre pas fournisseur/modèle par
rôle, parallélisme, tentatives, budgets ni tarifs — **par conception** :
`docs/T1-inventaire-existant.md:44,62-90` documente que ces quatre familles
(budgets, quotas, pricing, provider-admin) sont déjà administrées ailleurs
(`budgets.go`, `quotas.go`, `pricing.go`, `provider_admin.go`) avec une seule
source de vérité, et que `RunLimits` est spécifiquement l'objet qui comblait un
vide (aucune surface admin dédiée avant cette mission). Cette frontière de
périmètre n'est pas une lacune de cette tentative ; elle n'a pas été rouverte
faute de critère assigné dessus. Je ne l'ai pas corrigée moi-même : aucun des
critères req-4..req-7 ne porte sur la structure du document, et le contrat de
tâche ne m'autorise pas à étendre le périmètre sans décision explicite.

## Limites

- Aucune modification de fichier applicatif par cette tentative ; uniquement
  re-vérification et mise à jour du présent rapport.
- Dernier départ disponible (3/3) sur cette tâche : le responsable doit statuer
  (acceptation, ou nouvelle tâche/plan si un point reste jugé insuffisant) —
  aucune relance automatique possible côté worker.
- La revue indépendante et la gate de livraison (« Revue indépendante du
  contrat moteur et gate fraîche avant Lots 2 et 3 ») n'ont pas été exécutées
  par cette tentative ; ce n'est pas dans le périmètre d'un worker de
  s'auto-accepter.
- Suite complète (`go test ./... -count=1`) non rejouée entièrement par cette
  tentative (déjà rejouée et verte par le rapport du 29/09, ~368s) ; seuls les
  tests ciblés `TestRunLimits*` ont été rejoués ce jour, plus `build`/`vet` sur
  l'ensemble du module. Aucun échec, aucune régression détectée.

## Prochaine action

Responsable planner : évaluer si le constat non bloquant ci-dessus (duplication
du livrable avec le rapport de suivi) justifie une tâche de reformulation
documentaire distincte, ou si l'état actuel (preuves de test complètes et
fraîches sur les 4 critères) suffit pour la gate de livraison T2 avant Lots 2/3.
Aucune acceptation, aucune modification de plan ni de base Swarm effectuée par
cette tentative.

## Transmission des preuves au vérificateur — 30 septembre

Le moteur est maintenant autorisé depuis l’interface web à exécuter sept
contrôles Go ciblés, sans cache (`-count=1 -v`) dans des bases de test isolées :
plan-entry = TestRunLimitsMigrationKeepsExistingConfiguration ;
plan-criterion-1 = TestRunLimitsConfigRejectsInvalidValue ;
plan-criterion-2 = TestRunLimitsConfigPersistsAfterRealRestart ;
plan-criterion-3 = TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected ;
plan-criterion-4 = TestRunLimitsActualLaunchFreezesConfiguration ;
plan-validation = TestRunLimitsConfigConcurrencyAndReplay ;
plan-delivery = TestRunLimitsLaunchCannotRaiseCeilings.
Les reçus engine_controls doivent attester leur exécution, indépendamment
des anciennes captures du rapport. La migration n’est plus inférée d’une
liste de PASS antérieure incomplète : elle possède son propre contrôle.
Aucune nouvelle exécution de développement demandée pour cette transmission.
