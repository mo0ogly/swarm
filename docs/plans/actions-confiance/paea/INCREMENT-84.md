# PAEA84 — vérificateur PA-09 de rapprochement des journaux d'effets réels

Reprise du 29oct (continuité glm-resume-20261029T064841Z), service v246 actif
(ELF b23ac01e…), incrément 83 terminé et installé. Aucun commit/push, aucun
sous-agent, aucun appel payant, mission native suspendue préservée.

## Besoin

Le lot PA-09 exige que « le vérificateur rapproche décision et effet sans croire
le rapport agent ». Les deux consommateurs d'effet réels (settlement,
incrément 66 ; promotion Git, incrément 83) enregistrent des reçus durables
dans leurs journaux, mais aucun composant ne rapprochait ces effets de leurs
intentions durables par recomputation indépendante. L'audit Trust existant ne
couvre que les reçus simulés (`local-ledger-simulation`). Ce composant est
exigible sans les prérequis ouverts (séparation UID, enrôlement humain,
approbation indépendante) car il est lecture seule et n'accorde aucune
autorité.

## Réalisation (code)

Nouveau module `rust/crates/swarm-engine/src/root_store/paea_effect_reconcile.rs` :
`RootStore::paea_reconcile_effect_journals(&ledger)` — lecture seule, les deux
domaines réels (settlement sur le ledger séparé ; git-promote + intentions sur
le service courant). Par effet durable :

- Schéma vérifié avec les DDL EXACTS des producteurs (défaut corrigé : la 1re
  version recopiait les DDL de mémoire sans `CHECK(amount_minor>0)` ni
  `UNIQUE REFERENCES` → « Corrupt » faux ; cause établie, constantes réelles
  reprises). Absent = `Uninitialized` explicite ; altéré = `Corrupt{codes}`.
- Recanonisation indépendante : les octets d'intention stockés sont re-décodés
  strictement puis re-dérivés via `canonicalize` (paea-json-v1) ; digest,
  `intent_id` (clé métier), domaine (`Action::Payment` vs `Action::Git`) et
  octets doivent correspondre par recomputation.
- Settlement en plus : facture du paiement ↔ ligne d'effet, termes exacts de la
  dette provisionnée (`paea_settlement_debts`), forme des métadonnées
  d'autorité et horodatage RFC3339. Aucune fraîcheur attestée.
- Git en plus : jointure à l'intention durable réservée AVANT tout Git
  (`paea_git_intents`) ; colonnes du reçu (`target_ref`, `new_oid`) = colonnes
  de la ligne d'effet.

Types exportés (`PaeaEffectReconcileFinding/Journal/Report`, v1 sérialisable) :
`Uninitialized | Consistent{effect_count, findings} | Corrupt{codes}` ; chaque
finding porte les codes de refus exacts. Les incompatibilités sont rapportées,
jamais réparées ; `verified=true` n'établit que la correspondance recalculée,
pas l'authenticité (un écrivain même-UID peut réécrire tout le journal de façon
cohérente — prérequis ouverts conservés). Aucune autorité, publication,
mutation, provisionnement ou transport public.

`cargo check -p swarm-engine --tests` 0 erreur ; Rustdoc `-D warnings` propre.

## Qualification

`tests/paea_effect_reconcile.rs` : harnais settlement autonome (miroir
`paea_settlement.rs`) + harnais Git complet (miroir `paea_git_promote_call.rs`,
ordre métier réel transfert 82 → promotion 83). 3 tests `--ignored
--test-threads=1` PASS sur la fixture native vivante writer78b (9.4s) :

1. `reconcile_reports_consistent_journals_and_explicit_uninitialized` —
   settlement effect relié à son paiement re-dérivé + dette provisionnée ;
   effet Git relié à son intention durable + reçu exact ; journaux absents =
   `Uninitialized` explicite.
2. `reconcile_detects_altered_effect_digest_as_unverified_finding` — digest
   d'effet altéré par SQL direct sur le stockage de test possédé → finding non
   vérifié avec le code exact `paea-effect-reconcile-effect-digest`, dans les
   deux domaines.
3. `reconcile_reports_missing_git_intent_as_unverified` — intention durable
   supprimée → finding explicite `paea-effect-reconcile-intent-missing`, jamais
   un succès vide.

Recette permanente `tools/verification/recipes/paea-effect-reconcile.json`
(2 étapes : tests + Rustdoc, fixtures writer78b/current71c, sans watchdog
global).

## Documentation

Sections « Vérificateur PA-09 de rapprochement des journaux d'effets réels »
ajoutées à `docs/architecture/PAEA-GIT-TRANSACTION.md` et `docs/en/
PAEA-GIT-TRANSACTION.md` (contrat, limites, recette).

## Limites non modifiées

Aucun service accepté, aucune séparation UID, aucun enrôlement humain, aucune
approbation indépendante, aucune clôture globale. Le vérificateur est joignable
par l'API bibliothèque qualifiée en fixtures ; le raccordement au parcours
public (CLI/API/UX) reste un incrément séparé sous les prérequis réels. Il ne
remplace pas PA-09 complet (ancres externes, dossier de revue indépendant,
altération détectée par un observateur séparé restent ouverts).
