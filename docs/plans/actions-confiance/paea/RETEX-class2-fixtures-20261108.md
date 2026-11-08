# RETEX class-2 — migrations tempdir sous aires gouvernées, 8 novembre 2026

Rétrospective fondée sur les preuves de la session Claude du 7–8 novembre 2026
(lots D04 et class-2/D05). Portée : migration des 151 sites d'allocation nues
`tempfile::tempdir()`/`TempDir::new()` sous `rust/crates` vers les aires
gouvernées, extension du garde `tools/check_tmp_interdiction.py`, documentation
FR/EN. N'établit pas l'acceptation indépendante ; pas de commit/push (la
précédente autorisation a été consommée par `123eeeb0b`).

## 1. Résultat visé / obtenu

Visé : « reprends » — consigner et terminer la migration class-2 (D05) des
fixtures Rust vers les aires gouvernées, avec garde étendue et documentation
FR/EN dans le même incrément.

Obtenu : 151 sites migrés au contrat `tempdir_in(<aire>)`/`new_in(<aire>)`
(RAII `TempDir` conservé) — aire de dépôt `swarm_engine::fixture_area()`,
aire de repli `swarm_engine::fallback_area()` (`~/.local/state/swarm-fixtures/`,
0700) ou repli inline pour les crates sans dépendance swarm-engine. Garde
étendu : 0 violation / 10 items de revue. Compilation 0 erreur sur les 7
crates touchées. Vérification : enrollment+linux-prerequisites 110 verts,
authority 207 verts, engine `--lib` 750 verts / 6 rouges qualifiés hors
périmètre avec causes datées, cli intégral `--no-fail-fast` 258 verts / 9
rouges / 20 ignorés (les 9 = dettes préexistantes owner Codex), paea +29
verts avec fixture canonique. Docs FR/EN à jour. Preuves conservées sous
`rust/.swarm-fixtures/d0{4,5}-*`. **Non obtenu** : exécution des 20 tests
paea ignorés (recettes dédiées, binaires livrés, hors périmètre du lot) ;
reprise des 4 dettes consignées (owner Codex).

## 2. Séquence importante

1. Reprise après deux compactions : cartographie des 151 sites par crate,
   sélection d'aire par contrat (dépôt vs repli), première passe d'édition.
2. E0308 puis E0433 dans le helper d'aire inline de `authenticated_fixture.rs`
   (inclusion croisée `#[path]` depuis swarm-engine : `super::` ne résout pas
   le crate incluant) → helper autonome avec doc du cas.
3. 13 rouges `FixtureFault` plain `--tests` : feature-gated, préexistants à
   HEAD, hors parcours — requalification avec `--features test-fixtures`
   (convention des recettes projet), 0 erreur.
4. 7 rouges identity `--lib` : mon premier choix d'aire (dépôt) sous des
   modules dont le parcours traverse `open_private_directory` (refus des
   ancêtres 775 du checkout) → migration des 4 modules identity à l'aire de
   repli, 18 verts ; règle d'aire affinée et consignée.
5. Deux rouges engine déterministes : datés au lot Biba `2496c031b` (28 oct.)
   par reconstruction du code à l'origine du test (`174b89106`) et diff de
   chaîne — l'interposition `anchored_artifact::with` brise le downcast
   `LocalPathError`. Hors périmètre, owner vague Codex Biba.
6. trust_enrollment 2 rouges puis identity_checkpoint 6 rouges : même
   mécanique (le CLI enfant / `swarm_cli::run` lit ancres/déclarations via le
   walk privé) ; HEAD-before-blame par la cartographie BF-16b du 7 nov.
   (trust_enrollment y était vert) → 7 suites d'intégration migrées, 39 verts.
7. Premier run cli « complet » du 7 nov. sans `--no-fail-fast` : arrêt au
   premier échec, suites cli.rs/paea_settlement/planning jamais exercées ;
   run intégral du 8 nov. → 2 rouges cli.rs (même cause d'aire, corrigés,
   17 verts) et 1 rouge paea_settlement (même cause, corrigé) ; migration
   préventive des 4 suites paea à déclaration ; run final 258/9/20.
8. Garde étendu à « toute allocation tempfile Rust nue » (fenêtre
   `Builder…tempdir()` incluse) ; 947 résidus `.tmp*` sous /tmp tous
   antérieurs au lot (dernier 08:25 le 8), aucun leak depuis.

## 3. Mesures disponibles

- Sites : 151 migrés (engine 14, web 6, cli 86, enrollment+linux-prereq 51) ;
  23 sites fallback sous swarm-cli (1 version_no_repository, 4 modules
  identity `--lib`, 7 suites d'intégration, cli.rs, 5 suites paea).
- Tests : 110 + 207 + 750 + 258 verts ; 15 rouges qualifiés (7 identity
  `--lib` + 8 intégration + 3 cli/paea introduits puis retirés par le lot ;
  9 rouges résiduels = dettes Codex : 8 agent_launch BT02 Biba
  `biba-workspace-overlaps-root` — signature relue dans la preuve retenue
  `.tmpxUJ8Sl/active.stdout` — et 1 planning `agent-provider-catalogue-read`
  « jamais vert »).
- Garde : 0 violation / 10 items de revue (documentés/préexistants), 3 runs.
- Coût/temps : non mesurés (pas de compteur) — inconnus, pas nuls.

## 4. Ce qui a fonctionné

- La discipline HEAD-before-blame : chaque rouge a été rattaché à un état
  daté (cartographie BF-16b, `git show` au commit de création) avant toute
  inférence ; les 18 rouges introduits par le lot ont été distingués des 9
  dettes résiduelles sans ambiguïté.
- La règle d'aire objective dérivée de la mécanique : « tout parcours qui
  lit une ancre ou une déclaration via `open_private_directory` exige l'aire
  de repli, même sous swarm-cli » — appliquée préventivement aux suites paea
  restantes au lieu de découvrir les rouges une suite à la fois.
- L'interdiction /tmp tenue de bout en bout : aucun artefact de projet sous
  /tmp pendant les runs conformes ; TMPDIR gouverné
  (`/home/fpizzi/swarm-run-tmp`, 0700 hors dépôt) pour les suites qui
  l'exigent.

## 5. Mécanismes d'échec les plus coûteux (observés)

1. **Premier choix d'aire non discriminant (18 rouges en 3 vagues)** : la
   répartition initiale opposait « crates enrollment » à « swarm-cli »,
   or le critère réel est le parcours (walk privé) et non le crate. Cause :
   la cartographie initiale a classé les sites par crate sans tracer les
   parcours de lecture enfant. La règle finale est objective mais obtenue
   en trois itérations de rouges ; un tracé préalable des parcours
   (`open_private_directory` / `authenticated_startup::open` /
   `load_authenticated`) aurait donné la répartition correcte d'emblée.
2. **Run « complet » sans `--no-fail-fast` (fausse couverture)** : le run du
   7 nov. s'est arrêté à agent_launch et a laissé cli.rs, paea_settlement et
   planning jamais exercés ; le bilan « 80 verts / 8 rouges » de la
   cartographie était incomplet. Détecté le 8 par le run `--no-fail-fast`
   intégral. Leçon : un bilan de suite doit être `--no-fail-fast` ou porter
   la mention explicite « arrêt au premier échec, suites restantes non
   exercées ».
3. **Référence de preuve fantôme** : BLP-APEX citait `d05-guard-red.txt`
   jamais créé ; corrigé vers les fichiers réels (`d05-guard-final.txt`,
   `d05-cli-final-noff.txt`). Contrôle : citer uniquement des fichiers
   vérifiés par `ls` au moment de la rédaction.

## 6. Actions priorisées

| Action | Impact | Confiance | Effort | Propriétaire | Critère d'acceptation |
|---|---|---|---|---|---|
| ~~Tracer les parcours `open_private_directory`/`authenticated_startup::open` dans la cartographie des recettes permanentes~~ **Réalisé 8 nov.** : règle décisionnelle par parcours ajoutée au README de vérification (FR ligne ~168, EN ~218) — critère est le parcours, pas le crate ; applicable à une nouvelle suite sans test d'essai | Moyen | Haute | Faible | Fait | ✅ README FR/EN |
| Reprise des 9 rouges cli résiduels (8 agent_launch BT02 + 1 planning catalogue-read) | Haut | Haute | Moyen | Codex | Critère BIBA-TRANSVERSE-APEX.md ; le critère planning est à définir par le lot Codex (aucun fichier de référence vérifiable au 8 nov.) |
| Dette engine : downcast `LocalPathError` à travers `AnchoredArtifactError` (lot Biba `2496c031b`) + dérive v52 | Haut | Haute | Moyen | Codex | 2 rouges mission_status + 2 rouges settings verts au HEAD |
| ~~Intégrer `/home/fpizzi/swarm-run-tmp` comme TMPDIR gouverné des recettes permanentes~~ **Déjà réalisé, mieux** : le runner `recipe.py` épingle TMPDIR sous l'aire configurée (réglage Admin `runtime_temporary_directory`, repli privé opérateur — pas de chemin codé en dur) ; critère atteint et prouvé par la recette class-2 (PASS 4/4, VERIFIED, `environment.json` : `TMPDIR=~/.local/state/swarm-fixtures/runner-tmp`) | Moyen | Haute | Faible | Fait | ✅ Preuve `.swarm-fixtures/class2-recipe-20261108T030000Z` |
| Commit/push du lot (modifications locales préservées) | — | — | Faible | Utilisateur | Autorisation fraîche, puis commit sans `Co-Authored-By` |

## 7. Limites de ce RETEX

- Coût, temps et intervalle non mesurés (inconnus, pas nuls).
- La revue de ce lot n'est pas indépendante ; aucune acceptation indépendante
  n'a été obtenue.
- Les 20 tests paea ignorés et les recettes dédiées ne sont pas exercés par
  ce lot ; leur parcours à déclaration a été migré préventivement (critère
  objectif) mais la preuve d'exécution appartient aux recettes.
- Les lots antérieurs (v52, Biba) reçoivent leur qualification, pas leur
  correction ; owners consignés dans BLP-APEX FR/EN.
- Pas de commit/push ; tout reste modification locale préservée.
