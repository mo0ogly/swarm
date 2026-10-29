# PAEA92 — consommateur CLI gardé du settlement call

Continuité glm-resume-20261029T064841Z après l'incrément 91 (observation de
capacité web, installée v255). Limite levée : le settlement call était le
troisième appel gardé PAEA sans consommateur public — l'API bibliothèque
`call_paea_settlement` (PAEA-APEX incréments 80/81) n'était joignable que des
tests moteur ; un opérateur réel ne pouvait pas consommer un appel préparé par
le portail d'admission sans écrire son propre programme.

## Réalisé

- `rust/crates/swarm-cli/src/paea_settlement.rs` : module CLI deux phases
  `paea settlement-call commit|reconcile PROJECT LEDGER DECLARATION --input
  REQUESTFILE`. PROJECT ouvert en premier (rédacteur de la clôture IMMEDIATE
  qui fence le plan courant), LEDGER en second (racine consommatrice de
  l'effet) ; les deux témoins restent ouverts pendant toute la préparation.
  Préparation `project.paea_settlement_request(...)` → événement public
  `paea-settlement-prepared` (service `payment.settle`, `operation`,
  `root_digest`, `action_digest`, permission, `authority:false`,
  `restart_reusable:false`). Trame privée stdin
  `swarm-trust-source-v1\n<pin64>\n<password>` (grammaire partagée
  transfer/promote ; HEADER const, cut=HEADER.len()+64, pin64 validé par
  `swarm_identity_contract::digest`, stdin non terminal exigé, secret jamais
  émis sur aucune sortie). Consommation `ledger.call_paea_settlement(...)`
  sous autorité réelle tenue (`authenticated_startup::open` sur DECLARATION).
- Résultats publics : `paea-settlement-result` — `recorded` (intent_id,
  action_digest, invoice_id, trust_revision, commit_requested_at, replayed)
  ou `reconciled_not_committed` (intent_id, invoice_id). Reconcile = lecture
  pure du journal, jamais un nouveau grant.
- Fermetures : appariées sur chaque issue, causes chaînées dans
  `CliError::RootCleanup` via `failed(ledger, project, primary, closes)` ;
  échec de fermeture d'autorité = `RootStoreError::Refused
  ("paea-settlement-call-authority-close")` distinct, jamais un succès inventé.
- Câblage `swarm-cli/src/lib.rs` : `mod paea_settlement` (cfg linux), variante
  `CliError::PaeaSettlement(Box<PaeaSettlementCallError>)` (docstring, arms
  `code()`/`message()`/`source()` ; code `paea-settlement-action` porte les
  refus de préparation dont `paea-settlement-separate-ledger-required`),
  dispatch `settlement-call`, USAGE + HELP_FR/HELP_EN.

## Tests

`rust/crates/swarm-cli/tests/paea_settlement.rs` — CLI réel (binaire
`swarm-rust`, trame stdin) sur deux racines physiquement distinctes
(harnais modèle `tests/paea_settlement.rs` moteur : document canonique
`web/paea-examples/v1/payment-document.json`, ledger initialisé
`paea_initialize_settlement(project_id, "0b"*32, [proposal])`) :

- `settlement_call_cli_commits_once_and_records_effect` : commit exact une
  fois (`recorded`, `replayed:false`, `invoice-17`, effet durable compté = 1).
- `settlement_call_cli_reconciles_before_commit_reports_not_committed` :
  réconciliation négative avant commit (`reconciled_not_committed`, zéro
  ligne d'effet).
- `settlement_call_cli_refuses_bad_frame_without_consumption` : trame
  erronée refusée avant toute consommation (`paea-settlement-call-frame`,
  état Trust du projet inchangé, zéro ligne d'effet) ; aucun secret dans
  stdout/stderr.

3/3 PASS. Recette permanente
`tools/verification/recipes/paea-settlement-call-cli-rust.json` → PASS 3/3,
`recipe.py verify` → integrity VERIFIED ; preuves
`evidence/20261029-settlement-call-cli-mvfdts57`.

## Docs

- `docs/architecture/PAEA-GIT-TRANSACTION.md` § « Consommateur CLI gardé du
  settlement call (incrément 92) » + miroir EN (« Guarded CLI consumer of the
  settlement call (increment 92) »).
- Note version_history FR/EN de l'incrément 92.

## Limites restantes

La préparation CLI prépare ; elle n'authentifie personne et n'accorde rien —
la permission exacte est provisionnée hors bande et le recheck moteur sous
autorité reste l'arbitre à la consommation. Séparation UID, enrôlement humain,
approbation indépendante, PA-09 complet, PA-11/12 (E3), revue indépendante et
acceptation globale restent ouverts ; ils ne sont pas fabriqués ici. Aucun
commit/push ; installation locale v256 dans la suite immédiate du même
incrément.
