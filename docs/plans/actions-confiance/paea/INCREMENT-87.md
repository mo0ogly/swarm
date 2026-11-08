# PAEA87 — inspection Admin réelle + révision Trust et sérialisation des tests d'acquisition

Reprise 07:47 UTC du 2026-10-29 : dernier 86 clos (v250 installée), limite
déclarée « rendu pixel non vérifié ». Dossier de suivi
glm-resume-20261029T064841Z. Aucun commit/push, aucune notification externe.

## Partie 1 — inspection visuelle réelle du modal Admin (limite 86 levée)

Parcours navigateur réel puppeteer/Chrome système sur le service installé
v250 (127.0.0.1:38433) : login POST /login avec le token de session possédé
(`.swarm/web-session-e6ed55c9891a1038`) → cockpit → onglet Administration →
lien « Ouvrir les réglages Rust » (créé par `cockpit-native.js` setup) →
`/native-runtime.html?view=admin` → « Régler le moteur » → modal
`dialog[data-engine-settings]` ouvert. Assertions DOM : champ v49
`paea_settlement_ledger_root` rendu dans « Stockage et Git (5) », label FR
servi, déclencheur d'aide présent, capture PNG non vide (histogramme vérifié).
Preuves : `evidence/20261029-fixture-regen-mvfdts57/admin-visual-86/`
(admin-v49-field.png, result.json). Harnais borné
`tests/glm_admin_visual_check.cjs` (transport CDP, aucune autorité
d'acceptation). Points de méthode : `page.click` n'active pas le
gestionnaire `#tabs` du cockpit (dispatchEvent MouseEvent bubbles requis).
L'inspection reste une inspection, pas une revue indépendante ni une
acceptation globale.

## Partie 2 — révision Trust réelle dans les tests transfer call (défaut de test)

Reproducteur : `tests/paea_git_transfer_call.rs` (2 tests `--ignored` avec
`SWARM_PAEA_SOURCE_FIXTURE`) échouait `paea-git-transfer-trust-revision`.
Cause générale : le fichier code en dur la révision Trust signée **1** et
`expected_revision:0` dans la config, alors que le fixture retenu writer78b
porte la révision **47** ; le contrat produit (`paea_git_transfer_call.rs::policy`)
exige l'égalité entre la révision signée et l'état Trust actuel du projet.
Le test `paea_git_promote_call.rs` (déjà qualifié) lit la révision réelle
(`current_trust_revision`) — le transfer call a hérité d'une version antérieure.

Correction (motif promote/reconcile existant, aucune exception) : lecture de
la révision réelle après `config-apply`, `context(revision)` avec
`policy_revision` réel, signature `build_for_trust(..., trust_revision)`
et `Owned.trust_revision` — pour les deux tests (succès et refus
wrong-root). `effect_config` garde `expected_revision:0` : la première
application sur le fixture partage le même comportement idempotent que les
tests promote/reconcile (défauts d'installation à révision 1, configuration
déjà appliquée à la naissance du fixture). Résultat : 2/2 PASS avec le
fixture retenu réel, `--test-threads` par défaut ; le fixture Trust reste à
47 (aucune mutation résiduelle).

## Partie 3 — sérialisation du fixture source partagé (défaut de test)

Reproducteur stable : `tests/paea_git_repository.rs -- --ignored` en
parallèle (défaut du runner) → `git_source_context_revision_refuses_old_guard_and_recovers`
échoue `local-storage` sur `apply_settings` (ligne ~652). Cause générale,
produit correct : `apply_settings` prend la réservation exclusive
NON-bloquante du fixture partagé (sans retry, EAGAIN direct) tandis que les
tests voisins tiennent des réservations partagées sur la même racine via
`current_managed_git_publication`/`retain_source` (`with_material_read`).
Deux défauts de test distincts :
1. **Contention** : aucun ordre entre tests mutateurs du fixture partagé.
   Correction : mutex in-process `SOURCE_FIXTURE_MUTEX` pris par les deux
   tests touchant le fixture source (`git_source_owned_lane_rebinding_and_recovery`,
   `git_source_context_revision_refuses_old_guard_and_recovers`) ; les
   lecteurs contrôlés gardent leurs réservations partagées bornées.
2. **event_id non renouvelable** : le test réutilise l'event_id statique
   `source-context-revision`. Après un premier apply persisté (révision N),
   un second run construit le même event_id avec `expected_revision: N+1` :
   `replay()` trouve l'entrée avec un changement différent →
   `settings-event-conflict`. Correction : event_id lié à la révision
   observée `source-context-revision-{revision}` — chaque exécution est un
   changement réel, jamais un rejeu fabriqué ; le comportement rejouable du
   produit reste testé par la couche lib (`replay()`).

Résultat : 7/7 PASS en parallèle (défaut), deux exécutions consécutives
stables, historique du fixture croissant de façon saine
(`source-context-revision-{2,3}`), valeurs inchangées.

## Partie 4 — manifeste de recette source-context (régression manifeste)

`paea-native-git-source-context.json` (non suivi) exigeait les 3 variables
`SWARM_PAEA_*` que la réussite historique (core02 du 2026-10-28,
`evidence/20261028-source-writer78-in84w0yx/core02`, PASS) passait par
l'environnement hérité ; le manifeste actuel ne les déclarait plus → échec
avant toute exécution de test. Correction : les trois fixtures déclarés
dans `environment` (mêmes chemins que `paea-owned-git-transfer.json`).
Résultat : recette 6 étapes PASS, `recipe.py verify` →
`integrity: VERIFIED` (`evidence/20261029-fixture-regen-mvfdts57/
source-context-eventid-fix3`).

## Qualification

- `tests/paea_git_transfer_call.rs -- --ignored` : 2/2 PASS (fixture retenu,
  trust réel) — deux exécutions (une avec les 3 env, une avec le seul
  SOURCE requis, prouvant le besoin exact).
- `tests/paea_git_repository.rs -- --ignored` : 7/7 PASS en parallèle,
  stable sur rerun ; en `--test-threads=1` aussi.
- Recette `paea-owned-git-transfer-reconcile` (couvre les 2 tests transfer
  call + 9 états natifs + rustdoc) : 4 étapes PASS, VERIFIED
  (`transfer-reconcile-trustrev-fix`).
- Recette `paea-native-git-source-context` : 6 étapes PASS, VERIFIED
  (`source-context-eventid-fix3`).
- Échecs intermédiaires conservés comme preuves :
  `source-context-eventid-fix`/`-fix2` (manifeste sans fixtures), logs rtk
  des runs parallèles défaillants.

## Documentation et limites

- Rustdoc/commentaires des harnais et tests mis à jour (contrat du mutex,
  event_id lié à la révision, refs `SOURCE_FIXTURE_MUTEX`).
- INCREMENT-87 (ce fichier), APEX, PROGRESS (07:50 inspection, entrée 87).
- N'installe aucun binaire : aucun code produit (src/) modifié — les trois
  correctifs sont test-side et recipe-side ; v250 reste la version installée.
- N'établit ni PA-09 complet, ni revue indépendante, ni acceptation globale ;
  ancres externes PA-09 (clés trust/I3 admin, humains) restent ouvertes.
