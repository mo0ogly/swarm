# Avancement de la migration Go → Rust — 2026-10-24 11:11

> **Instantané historique, non représentatif de l’état actuel.** Relecture du24octobre1454 : ce document contient des états devenus faux (watch/installation) et des comptes de présence qui ne mesurent pas la charge restante. Consulter la [situation actuelle du plan](PLAN.md#situation-actuelle--24-octobre-correction-de-pilotage1454). Les tableaux suivants sont conservés pour leur provenance ; leur régénération doit être requalifiée avant réutilisation de taux ou de délais.

Généré par `tools/migration-progress/avancement.py` (relancer après chaque lot ; `--record` ajoute un point d'historique). Comptes **par élément, non pondérés par l'effort**. Niveaux cumulés : un élément « vérifié » compte aussi comme « existe ».

| Surface | Total Go | Existe en Rust | dont vérifié (preuve PASS citée) | Parité acceptée | Partiel (équivalent renommé) |
|---|---|---|---|---|---|
| CLI (opérations) | 161 | 37 (23 %) | 6 (4 %) | 0 (0 %) | 9 |
| Web (routes API) | 93 | 4 (4 %) | 0 (0 %) | 0 (0 %) | 0 |
| Web (pages) | 10 | 1 (10 %) | 0 (0 %) | 0 (0 %) | 0 |

**Données et bascule** (copié de `PLAN.md`, non calculé) :

- Schéma37 accepté côté Rust ; migrations1..36 non portées. Bundle Work Go ≠ snapshot complet Root. Build/install historiques encore Go ; aucune bascule faite.

| Lot | État | Condition restante |
|---|---|---|
| R0 | partiel | référence finale à figer |
| R1/R2 | partiels | données et migrations complètes |
| R3/R4 | partiels | moteur, supervision et autorités complets |
| R5 | partiel | CLI/API/UX complets et vérifiés |
| R6 | préparation commencée, recette non exécutée | migration/restauration et qualification globale |
| R7 | préparation commencée, aucune installation | livraison/rollback et RETEX sous autorité |

## Vitesse observée (2026-10-23T23:05 → 2026-10-24T11:11)

| Surface | Gagnés « existe » | Par jour | Restant | Projection naïve (estimation) |
|---|---|---|---|---|
| CLI (opérations) | 1 | 2.0 | 124 | ≈ 63 jours |
| Web (routes API) | 0 | 0.0 | 89 | non calculable (aucun gain) |
| Web (pages) | 0 | 0.0 | 9 | non calculable (aucun gain) |

Projection linéaire sur la seule présence : elle ignore la parité, la migration des données, R6/R7 et la difficulté inégale des éléments.

## Taille du code par domaine (Rust / Go, hors tests et commentaires)

| Domaine | Go (lignes) | Rust (lignes) | Rapport Rust/Go | Évolution depuis le premier relevé |
|---|---|---|---|---|
| Télémétrie et console live (nouveau) | — | 3030 | sans équivalent Go | — |
| Relecture (review) | 11446 | 6035 | ×0.5 | 0.5 → 0.5 |
| Planification | 3671 | 8060 | ×2.2 | 2.2 → 2.2 |
| Mission (conducteur, file, reprise) | 3673 | 11954 | ×3.3 | 3.0 → 3.3 |
| Agents (lancement, supervision) | 9143 | 14555 | ×1.6 | 1.6 → 1.6 |
| Budget, coûts, quotas | 1253 | 2141 | ×1.7 | 1.7 → 1.7 |
| Fournisseurs | 1305 | 2234 | ×1.7 | 1.7 → 1.7 |
| Confiance, identité, enrôlement | 10520 | 4000 | ×0.4 | 0.4 → 0.4 |
| Préparations | 856 | 0 | ×0.0 | 0.0 → 0.0 |
| Automatisation | 2214 | 0 | ×0.0 | 0.0 → 0.0 |
| Mémoire, projet | 2270 | 1387 | ×0.6 | 0.6 → 0.6 |
| Réglages | 770 | 2213 | ×2.9 | 2.9 → 2.9 |
| Travaux et tâches | 1931 | 6232 | ×3.2 | 3.2 → 3.2 |
| Web, HTTP, transport | 2043 | 3540 | ×1.7 | 1.6 → 1.7 |
| Autres | 25774 | 17281 | ×0.7 | 0.7 → 0.7 |
| **Total (hors nouveau)** | 76869 | 79632 | ×1.0 | |

Le rapport compare des volumes, pas des fonctions : un domaine encore peu porté a un rapport bas, un domaine porté avec beaucoup de code défensif ou de durcissement un rapport haut. Rattachement des fichiers par mots-clés de chemin (heuristique, voir `DOMAINS`).

## CLI (opérations)

| Élément | Niveau | Preuve ou équivalent |
|---|---|---|
| `agent logs` | verifie | docs/plans/migration-rust/evidence/20261023-g1-status-contract/decision563-logs/result.json |
| `mission status` | verifie | docs/plans/migration-rust/evidence/20261022-root-instance-201/watch-public213/result.json |
| `mission watch` | verifie | docs/plans/migration-rust/evidence/20261022-root-instance-201/watch-public213/result.json, docs/plans/migration-rust/evidence/20261022-g3-watch-restart/public224/result.json, docs/plans/migration-rust/evidence/20261022-g3-watch-planning/public244/result.json, docs/plans/migration-rust/evidence/20261023-g1-status-contract/recipe657-controls/result.json, docs/plans/migration-rust/evidence/20261023-g1-status-contract/recipe658-revocation/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe696-mission-review-refusal/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe697-mission-report-once/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe698-mission-controls-revocation/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe907-manual-automatic-controls/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe908-review-refusal/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1006-foreground-signals/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1007-manual-controls/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1007-review-refusal/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1028-registered-observer/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1054-async-planner/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1054-registered-observer/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1055-manual-controls/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1055-review-refusal/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1067-async-roles/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1067-registered-observer/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1070-manual-controls/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1070-review-refusal/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1071-controls-revocation/result.json, docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe1071-reviewer-revocation/result.json |
| `version` | verifie | docs/plans/migration-rust/evidence/20261024-g3-auto-review/recipe906-native-version/result.json |
| `work list` | verifie | docs/plans/migration-rust/evidence/20261022-root-instance-201/differential-public194/bench/result.json |
| `work show` | verifie | docs/plans/migration-rust/evidence/20261022-root-instance-201/differential-public194/bench/result.json |
| `agent reconcile` | existe |  |
| `agent show` | existe |  |
| `agent stop` | existe |  |
| `budget apply` | existe |  |
| `checkpoint` | existe |  |
| `evaluate` | existe |  |
| `gate` | existe |  |
| `identity bootstrap-check` | existe |  |
| `init` | existe |  |
| `ooda` | existe |  |
| `planning claim` | existe |  |
| `planning decide` | existe |  |
| `planning enable` | existe |  |
| `planning pause` | existe |  |
| `planning resume` | existe |  |
| `planning show` | existe |  |
| `planning step` | existe |  |
| `pricing estimate` | existe |  |
| `pricing list` | existe |  |
| `pricing save` | existe |  |
| `quotas apply` | existe |  |
| `quotas preview` | existe |  |
| `quotas show` | existe |  |
| `resume` | existe |  |
| `task add` | existe |  |
| `task update` | existe |  |
| `task-model apply` | existe |  |
| `task-model preview` | existe |  |
| `task-model show` | existe |  |
| `work create` | existe |  |
| `work update` | existe |  |
| `agent context-preview` | partiel | équivalent : agent preview |
| `agent preflight` | partiel | équivalent : agent preview (not Go preflight DTO) |
| `agent start` | partiel | équivalent : agent prepare + agent run (two distinct operations) |
| `mission pause` | partiel | même nom, contrat partiel ou différent (matrice) ; équivalent : mission pause |
| `mission resume` | partiel | même nom, contrat partiel ou différent (matrice) ; équivalent : mission resume |
| `mission start` | partiel | même nom, contrat partiel ou différent (matrice) ; équivalent : mission start |
| `mission stop` | partiel | même nom, contrat partiel ou différent (matrice) ; équivalent : mission stop |
| `providers init` | partiel | équivalent : providers config init |
| `providers show` | partiel | équivalent : providers config show |
| `agent list` | absent |  |
| `agent prepared` | absent |  |
| `agent resume-launch` | absent |  |
| `aide` | absent |  |
| `automation archive` | absent |  |
| `automation cancel` | absent |  |
| `automation create` | absent |  |
| `automation enable` | absent |  |
| `automation list` | absent |  |
| `automation params` | absent |  |
| `automation pause` | absent |  |
| `automation preview` | absent |  |
| `automation show` | absent |  |
| `autonomy` | absent |  |
| `budget preview` | absent |  |
| `budget show` | absent |  |
| `connections list` | absent |  |
| `connections save` | absent |  |
| `console` | absent |  |
| `control` | absent |  |
| `dispatch` | absent |  |
| `doctor` | absent |  |
| `exchange consume` | absent |  |
| `exchange list` | absent |  |
| `exchange send` | absent |  |
| `export` | absent |  |
| `import` | absent |  |
| `lifecycle apply` | absent |  |
| `lifecycle list` | absent |  |
| `lifecycle preview` | absent |  |
| `memory admit` | absent |  |
| `memory admit-preview` | absent |  |
| `memory list` | absent |  |
| `memory replace` | absent |  |
| `memory replace-preview` | absent |  |
| `memory select` | absent |  |
| `memory select-preview` | absent |  |
| `memory summary-source` | absent |  |
| `memory withdraw` | absent |  |
| `memory withdraw-preview` | absent |  |
| `mission changes` | absent |  |
| `mission preview` | absent |  |
| `mission recovery` | absent |  |
| `mission seen` | absent |  |
| `mission spending` | absent |  |
| `plan draft apply` | absent |  |
| `plan draft compare` | absent |  |
| `plan draft export` | absent |  |
| `plan draft import` | absent |  |
| `plan draft preview` | absent |  |
| `plan draft redo` | absent |  |
| `plan draft show` | absent |  |
| `plan draft undo` | absent |  |
| `planning authorize-recovery` | absent |  |
| `planning handoff` | absent |  |
| `planning requalify` | absent |  |
| `planning revise-recovered-result` | absent |  |
| `planning submit-recovered-result` | absent |  |
| `prepare adopt-brief` | absent |  |
| `prepare create` | absent |  |
| `prepare export` | absent |  |
| `prepare history` | absent |  |
| `prepare list` | absent |  |
| `prepare method` | absent |  |
| `prepare methods` | absent |  |
| `prepare save` | absent |  |
| `prepare show` | absent |  |
| `prepare validate-plan` | absent |  |
| `profile` | absent |  |
| `project-profile check` | absent |  |
| `project-profile list` | absent |  |
| `project-profile select` | absent |  |
| `project-profile show` | absent |  |
| `providers cooldown clear` | absent |  |
| `providers cooldown show` | absent |  |
| `providers relay decide` | absent |  |
| `providers relay show` | absent |  |
| `role-model apply` | absent |  |
| `role-model preview` | absent |  |
| `role-model show` | absent |  |
| `run-limits apply` | absent |  |
| `run-limits effective` | absent |  |
| `run-limits history` | absent |  |
| `run-limits rollback` | absent |  |
| `run-limits show` | absent |  |
| `skills list` | absent |  |
| `trust anchor` | absent |  |
| `trust approve` | absent |  |
| `trust commit` | absent |  |
| `trust config-apply` | absent |  |
| `trust config-preview` | absent |  |
| `trust confinement-check` | absent |  |
| `trust direct-write` | absent |  |
| `trust elevate-integrity` | absent |  |
| `trust export` | absent |  |
| `trust identity-context` | absent |  |
| `trust mandate-create` | absent |  |
| `trust mandate-revoke` | absent |  |
| `trust oidc-backchannel-logout` | absent |  |
| `trust oidc-finish` | absent |  |
| `trust oidc-logout` | absent |  |
| `trust oidc-session` | absent |  |
| `trust oidc-start` | absent |  |
| `trust reconcile` | absent |  |
| `trust request` | absent |  |
| `trust show` | absent |  |
| `trust verify` | absent |  |
| `trust workload-login` | absent |  |
| `trust workload-logout` | absent |  |
| `validation apply` | absent |  |
| `validation preview` | absent |  |
| `validation recheck-apply` | absent |  |
| `validation recheck-preview` | absent |  |
| `workspace integrate` | absent |  |
| `workspace status` | absent |  |

## Web (routes API)

| Élément | Niveau | Preuve ou équivalent |
|---|---|---|
| `/api/v1/action` | existe |  |
| `/api/v1/planning` | existe |  |
| `/api/v1/quotas` | existe |  |
| `/api/v1/works` | existe |  |
| `/api/v1/activity` | absent |  |
| `/api/v1/agent-detail` | absent |  |
| `/api/v1/assist/ask` | absent |  |
| `/api/v1/assist/cancel` | absent |  |
| `/api/v1/assist/context` | absent |  |
| `/api/v1/assist/export` | absent |  |
| `/api/v1/assist/meta` | absent |  |
| `/api/v1/assist/turns` | absent |  |
| `/api/v1/automation/cancel` | absent |  |
| `/api/v1/automation/config` | absent |  |
| `/api/v1/automation/create` | absent |  |
| `/api/v1/automation/external` | absent |  |
| `/api/v1/automation/preview` | absent |  |
| `/api/v1/automation/programs` | absent |  |
| `/api/v1/automation/state` | absent |  |
| `/api/v1/control-diagnostic` | absent |  |
| `/api/v1/dialogue-resume` | absent |  |
| `/api/v1/dialogue-resume-preview` | absent |  |
| `/api/v1/events` | absent |  |
| `/api/v1/exchanges` | absent |  |
| `/api/v1/graph-drafts` | absent |  |
| `/api/v1/graph-drafts/apply` | absent |  |
| `/api/v1/graph-drafts/preview` | absent |  |
| `/api/v1/graph-performance` | absent |  |
| `/api/v1/graph-performance/apply` | absent |  |
| `/api/v1/graph-performance/preview` | absent |  |
| `/api/v1/lifecycle/missions` | absent |  |
| `/api/v1/logs` | absent |  |
| `/api/v1/planning-bundle` | absent |  |
| `/api/v1/planning-cleanup` | absent |  |
| `/api/v1/planning-history` | absent |  |
| `/api/v1/preparation-timeout` | absent |  |
| `/api/v1/preparation-timeout/apply` | absent |  |
| `/api/v1/preparation-timeout/preview` | absent |  |
| `/api/v1/preparations` | absent |  |
| `/api/v1/preparations/budget` | absent |  |
| `/api/v1/preparations/command` | absent |  |
| `/api/v1/preparations/conversion` | absent |  |
| `/api/v1/preparations/dialogue` | absent |  |
| `/api/v1/preparations/files` | absent |  |
| `/api/v1/preparations/history` | absent |  |
| `/api/v1/preparations/methods` | absent |  |
| `/api/v1/preparations/preflight` | absent |  |
| `/api/v1/preparations/providers` | absent |  |
| `/api/v1/preparations/send` | absent |  |
| `/api/v1/preparations/show` | absent |  |
| `/api/v1/preparations/source` | absent |  |
| `/api/v1/preparations/stop` | absent |  |
| `/api/v1/preparations/template-check` | absent |  |
| `/api/v1/preparations/templates` | absent |  |
| `/api/v1/pricing` | absent |  |
| `/api/v1/pricing/estimate` | absent |  |
| `/api/v1/processes` | absent |  |
| `/api/v1/project-memory` | absent |  |
| `/api/v1/project-profile` | absent |  |
| `/api/v1/project-profile/catalog` | absent |  |
| `/api/v1/project-profile/select` | absent |  |
| `/api/v1/provider-wait` | absent |  |
| `/api/v1/provider-wait/apply` | absent |  |
| `/api/v1/provider-wait/preview` | absent |  |
| `/api/v1/providers/admin` | absent |  |
| `/api/v1/providers/connections` | absent |  |
| `/api/v1/providers/connections/test` | absent |  |
| `/api/v1/providers/policies` | absent |  |
| `/api/v1/providers/resolve` | absent |  |
| `/api/v1/recovery-preview` | absent |  |
| `/api/v1/report` | absent |  |
| `/api/v1/retex-evolution` | absent |  |
| `/api/v1/run-limits` | absent |  |
| `/api/v1/run-limits/apply` | absent |  |
| `/api/v1/run-limits/effective` | absent |  |
| `/api/v1/run-limits/preview` | absent |  |
| `/api/v1/run-limits/rollback` | absent |  |
| `/api/v1/runtime-health` | absent |  |
| `/api/v1/skills` | absent |  |
| `/api/v1/snapshot` | absent |  |
| `/api/v1/task` | absent |  |
| `/api/v1/terminal` | absent |  |
| `/api/v1/terminal/control` | absent |  |
| `/api/v1/trust-actions` | absent |  |
| `/api/v1/trust-actions/oidc-backchannel-logout` | absent |  |
| `/api/v1/trust-evidence` | absent |  |
| `/api/v1/trust-laboratory` | absent |  |
| `/api/v1/trust-secrets` | absent |  |
| `/api/v1/visit` | absent |  |
| `/api/v1/web-requests` | absent |  |
| `/api/v1/web-requests/apply` | absent |  |
| `/api/v1/web-requests/preview` | absent |  |
| `/api/v1/web-requests/script.js` | absent |  |

## Web (pages)

| Élément | Niveau | Preuve ou équivalent |
|---|---|---|
| `access.html` | existe |  |
| `index.html` | absent |  |
| `log-console.html` | absent |  |
| `prepare.html` | absent |  |
| `processes.html` | absent |  |
| `project-memory.html` | absent |  |
| `projects.html` | absent |  |
| `terminal.html` | absent |  |
| `trust-actions.html` | absent |  |
| `trust-oidc-callback.html` | absent |  |

## Routes Rust sans homonyme Go (non comptées)

`/api/v1/admin-advice-budget`, `/api/v1/admin-advice-budget/apply`, `/api/v1/admin-advice-budget/preview`, `/api/v1/live/agent`, `/api/v1/live/alerts`, `/api/v1/live/alerts/evaluate`, `/api/v1/live/annotations`, `/api/v1/live/dataset`, `/api/v1/live/events`, `/api/v1/live/fingerprint`, `/api/v1/live/overview`, `/api/v1/live/run`, `/api/v1/live/scenario`, `/api/v1/mission`, `/api/v1/native-capabilities`, `/api/v1/native-session`, `/api/v1/native-settings`, `/api/v1/native-settings/apply`, `/api/v1/native-settings/describe`, `/api/v1/native-settings/preview`, `/api/v1/quotas/apply`, `/api/v1/quotas/preview`

Certaines remplacent une route Go sous un autre nom (`native-*`) ; tant que la matrice ne le consigne pas, elles ne sont pas comptées.

## Opérations Rust sans homonyme Go (non comptées)

`admin-budget apply`, `admin-budget preview`, `admin-budget show`, `agent guard`, `agent limits`, `agent prepare`, `agent preview`, `agent run`, `agent-workflow`, `costs show`, `engine-settings apply`, `engine-settings describe`, `engine-settings model-check`, `engine-settings preview`, `engine-settings quality-profile`, `engine-settings show`, `mission order`, `mission priority`, `mission queue`, `mission show`, `mission status-native`, `mission unpause`, `planning call-preview`, `planning judge`, `planning lease-renew`, `planning reconcile`, `planning renew`, `planning run`, `planning-output check`, `planning-report`, `project-context`, `providers config apply`, `providers config backups`, `providers config init`, `providers config inspect`, `providers config preview`, `providers config show`, `providers policies apply`, `providers policies export`, `providers policies preview`, `task review`, `task review-inputs`, `task review-preview`, `task review-reconcile`, `task review-retire-stale`, `task submit`, `task validate`, `task validation-preview`, `task validation-reconcile`, `task validation-retire`, `web`

Sources : aide de `swarm`, aide de `.cache/rust-migration-binaries/swarm-rust-9e565a05988f7a2b3843b730645f47421a71f4afdfa7475e605a6f86f5ade8db` (sha256 9e565a05988f…), matrice `docs/plans/migration-rust/evidence/20261022-parallel-196/matrix-integrated.json`, `route()` de `service.rs`, pages `web/*.html`.
