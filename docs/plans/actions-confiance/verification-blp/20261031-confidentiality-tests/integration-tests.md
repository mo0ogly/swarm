# Preuves test d'intégration BLP swarm-engine — 31 octobre 2026

Commande (depuis `rust/`, TMPDIR = aire de fixtures projet exigée par le test) :

```
TMPDIR=/home/fpizzi/Documents/ChatGPT/swarm/.swarm-fixtures \
cargo test -p swarm-engine --test confidentiality -- --test-threads=2
```

Résultat final : **ok. 5 passed; 0 failed; 0 ignored; 0 measured** (1,32 s).
Rejoué identiquement sous le repli privé opérateur
`TMPDIR=/home/fpizzi/.local/state/swarm-recipe-fallback-fixtures/runner-tmp`
→ 5/5 PASS (1,34 s) ; les tests BLP passent dans les deux zones contractuelles.

Tests nommés (liste `-- --list`) :
- conflicts_catalog_changes_and_unrecognized_grants_do_not_mutate
- policy_preview_apply_reopen_and_replay_preserve_history
- session_local_operator_or_forged_level_cannot_configure_blp
- supplied_label_diagnostics_are_readonly_and_never_runtime_authority
- trust_revision_policy_revocation_and_old_replays_are_not_reactivated

## Correctif du test requis pour ce PASS

Échec initial conservé : les 5 tests paniquaient à
`tests/confidentiality.rs:20` (`config-apply` initial → erreur `"format"`).
Cause établie : `prepare_identities` (`trust_types.rs:157`) refuse
`inputs.len() < 3` ; le fixture fournissait 2 identités. Invariant métier
existant, pas un défaut moteur. Correction : troisième identité `approver`
(I2, jeton distinct) ajoutée au fixture, conforme au motif de `trust_biba.rs`.
Le fixture reste privé et isolé (0700, aire projet, sans base de mission).

## Empreintes des sources testées (SHA-256)

- tests/confidentiality.rs : 99a212837258977013255a84d133c5144ccc85622dd96a5de3befd75f1681e7a
- src/trust/confidentiality.rs : 920eeccb7317d18f822662de7622d6b07c2c69f3bb0a8cc2bdaa2a5186fc215f
- src/service/trust.rs (swarm-web, routage corrigé) : 183f5eab1e6f0249e8d05801cb11fdc39c8823aa5a4c2fda13314b2e9900cf99

## Compilations préalables (correctifs vérifiés)

- E0308 `CompactJson::new` : `cargo test -p swarm-engine --test trust_biba --no-run`
  → Finished, exécutable produit (correctif `from_utf8` du prédécesseur vérifié).
- Routage HTTP : `cargo build -p swarm-web` → Finished (57,64 s).
  Correctif : branche GET/HEAD rétablie à `"show"` seul ; opérations BLP
  ajoutées à la liste `matches!` des POST autorisés dans `service/trust.rs`.

## Portée

Preuves d'administration de politique et diagnostics data-only sur racines
privées. N'établit ni classification des contenus réels, ni exécution protégée
(`runtime_enforced:false` conservé), ni routage HTTP de bout en bout (le
parcours HTTP réel reste à qualifier avec le service installé), ni interface web.

## Régressions des suites trust existantes (consommateurs du même contrat)

Commandes depuis `rust/`, `--test-threads=2` :
- `--test trust_biba` → **ok. 14 passed; 0 failed** (2,83 s) — TMPDIR aire projet.
- `--test trust_governance` → **ok. 11 passed; 0 failed** (4,57 s) — TMPDIR aire projet.
- `--test trust_enrollment` → 3/3 PASS (3,60 s) **uniquement** sous le repli
  privé `~/.local/state/swarm-recipe-fallback-fixtures/runner-tmp` ;
  sous `.swarm-fixtures` direct, échec `enrollment config: path` — cause établie :
  le fixture tiers `swarm-enrollment-authority/tests/common/authenticated_fixture.rs`
  (`tempfile::tempdir()`, ligne 99) vérifie un contrat de répertoire privé
  (`open_private_directory` refuse un ancêtre inscriptible groupe) ; le README
  des recettes prescrit précisément ce repli pour ces fixtures. Défaut de choix
  d'environnement de vérification, pas de code ; fichier tiers non modifié.
