# PAEA86 — consommateur web public du vérificateur PA-09

Continuité glm-resume-20261029T064841Z, service v248 actif (ELF
`957a3bfe…`), incrément 85 clos (recette PASS/VERIFIED, installation
vérifiée). Aucun commit/push, aucun sous-agent, aucun appel payant, mission
native suspendue préservée.

## Besoin

Après l'incrément 85 (CLI), le vérificateur PA-09 n'avait toujours aucun
parcours web/UX : PA-08 (parcours réel) et l'interface Admin exigent un accès
web en lecture seule. Ce consommateur est exigible sans les prérequis ouverts
(séparation UID, enrôlement humain, approbation indépendante) car il n'exécute
aucun effet, n'accorde aucune autorité et ne transporte rien publiquement.
Contrairement au CLI, la racine du grand livre séparé ne peut pas être un
argument d'appel : elle doit être une configuration d'opérateur persistée
(politique : source persistée → validation → Admin/API → consommateur), ce qui
ajoute le réglage versionné v49.

## Réalisation (code)

- **Réglage v49 `paea_settlement_ledger_root`** (motif exact v48
  `cockpit_default_autonomy`) : champ `Option<String>` dans
  `EngineSettingsDocument`, `schema_version()` = 49 en tête,
  `validate()` → nouveau module `engine_settings/settlement_ledger.rs`
  (`paea_settlement_ledger_policy`) : absence →
  `settings-paea-ledger-root-not-configured` ; forme invalide (vide, NUL, non
  absolu, slash final redondant, composants vides/`.`/`..`) →
  `settings-paea-ledger-root-invalid`. Le réglage est pure configuration : il
  ne désigne aucun grand livre initialisé et n'établit pas la séparation,
  vérifiée à chaque appel. Catalogue v6 (174 champs, unit `text`) et défauts
  d'installation mis à jour (contrainte `introduced_schema_version`).
- **Endpoint web** (`swarm-web/src/service/paea_reconcile.rs`) :
  `GET|HEAD /api/v1/paea/reconcile-effects`, session authentifiée obligatoire,
  lecture seule : corps ou requête non vide refusés
  (`paea-reconcile-body-invalid` / `paea-reconcile-query-invalid`) sans ouvrir
  de racine. Ouverture racine service (lecture seule) → politique du réglage →
  ouverture grand livre (lecture seule) →
  `paea_reconcile_effect_journals` → rapport JSON v1 ; fermetures appariées
  `close(result, cleanup)`. Un grand livre désigné non initialisé produit un
  rapport explicite `uninitialized`, pas une erreur.
- **Mapping de statut HTTP** : `409` pour
  `paea-effect-reconcile-separate-ledger-required` (opération `reconcile`) ;
  `501` pour `settings-paea-ledger-root-not-configured` et
  `settings-paea-ledger-root-invalid` (opération `settings-ledger-root`).
- **Admin UI** (`frontend/engine-settings/admin.js`) : le champ rejoint le
  groupe « Stockage et Git » (regex étendue `paea_settlement_`) ; deux
  étiquettes d'erreur FR ajoutées. Traductions EN : la revue de couverture a
  révélé des lacunes réelles corrigées dans `locales/en.json` — les 2
  étiquettes d'erreur, le label + help v49, ET deux couples label+help
  préexistants manquants (`report_preview_max_bytes`,
  `report_verification_max_bytes`) qui auraient échoué au preflight du harnais
  Admin (vérifié : 115 littéraux `tr()` + 174 champs catalogue couverts).
- **Défaut préexistant corrigé** (`swarm-web/tests/live.rs:571`) : macro
  `assert_eq!(logs.len());` malformée (committée dans 2496c031b) bloquant la
  compilation de toute la suite de tests web → comparateur restauré
  `TEST_LOG_PAGE_ROWS` (100 = `LIVE_PAGE_ROWS`).

## Qualification

- `cargo test -p swarm-engine --lib engine_settings` : **17 passed** (5
  nouveaux tests de politique v49 : schéma 49 sur défaut présent, schéma 48
  sur absence historique, chemins valides préservés, traversée/redondance
  refusées, null/non-chaîne refusés au décodage).
- `cargo test -p swarm-web --test paea_reconcile` : **3 passed** — rapport
  `uninitialized` via l'API authentifiée sans mutation du disque (octets DB
  avant==après, configuration par le vrai contrat preview+apply), refus 409 de
  racine partagée (code exact `paea-effect-reconcile-separate-ledger-required`),
  refus 403 sans session + refus POST/corps/requête sans IO.
- `cargo test -p swarm-web --test live agent_route_pages` : 1 passed (défaut
  de compilation préexistant corrigé).
- Catalogue : tests `engine_settings_catalog` à jour (schéma document 49,
  catalogue v6).
- Recette permanente `tools/verification/recipes/paea-effect-reconcile.json`
  étendue : inputs `swarm-web/**/*`, `locales/en.json`, `frontend/
  engine-settings/*` ; nouvelles étapes `effect-reconcile-web-consumer` (3
  tests web) et `effect-reconcile-settings-policy` (lib settings). Exécution :
  `docs/plans/actions-confiance/paea/evidence/20261029-fixture-regen-mvfdts57/
  reconcile-effect-86` (statut et intégrité relevés dans PROGRESS.md).

## Documentation

`docs/architecture/PAEA-GIT-TRANSACTION.md` et `docs/en/PAEA-GIT-TRANSACTION.md`
: contrat du consommateur web (endpoint, réglage v49, statuts 409/501, rapport
uninitialized), recette à jour (4+3+3 tests + lib settings). Rustdoc
swarm-engine/swarm-web à jour.

## Limites non modifiées

Le consommateur web reste une qualification locale : il n'établit ni
authenticité des journaux, ni séparation UID, ni enrôlement humain, ni
approbation indépendante, ni PA-09 complet (ancres externes, revue
indépendante, observateur séparé restent ouverts). L'inspection visuelle de la
surface Admin modifiée a été réalisée sur le service installé v250 (parcours
navigateur réel, champ v49 rendu dans « Stockage et Git (5) », capture
evidence/20261029-fixture-regen-mvfdts57/admin-visual-86/, PROGRESS.md
07:50 UTC) ; elle reste une inspection, pas une revue indépendante ni une
acceptation globale. L'installation locale est tracée dans PROGRESS.md.
Aucun service accepté ; PA02/03 partiels, 0/17, PA13/RETEX final ouverts.
