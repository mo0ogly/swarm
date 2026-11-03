# Tableau de bord PAEA — 1er novembre 2026

Instantané d'état construit depuis les registres réels par `/apex`. Sources
complètes en fin de fichier. Rappel : ce fichier est un instantané sourcé,
pas une revendication d'acceptation ; les cas locaux de PA-01 ne sont pas les
campagnes T01…T22 exécutées.

## Candidat et blocage courant

- Binaire désigné : `bin/swarm` SHA-256
  `e2f0039a1f3a34957c161e770c4e5efdd46f4e3a16fc82fe9cf5d9761dc94da3`
  (v0.1.1-dev.316, HEAD 562953dba3f8d6eaabca7beda35535b31a9390fa, posé
  2026-11-02 22:10:33 — 10e remplacement depuis dev.292 ci-dessous ;
  historique dev.315/186cc38b… et dev.292/4164953c… conservés pour contexte)
  — re-vérifié deux fois : worktree gelé 562953dba3 (settlement run-15 7/7 +
  chaîne chain-run-8 4/4) puis arbre vivant recompilé (settlement run-16 7/7 +
  chaîne chain-run-9 4/4) — les quatre VERIFIED (extensions 02-11 soirée et
  fin de nuit ci-dessous)
- Déclaration E2 bornée : **émise** le 31 octobre 2026
  (`claude-e2-arbitrage-20261031T135859Z/DECLARATION-E2.md`) — règlement 6/6 et
  chaîne 4/4 PASS VERIFIED sur fixture regen-5 ; clôt l'étape PA-10 seulement.
  Elle désigne l'ELF dev.267 ; les preuves ci-dessous re-pinnées sont celles
  re-jouées sur le livré courant.
- Critères ouverts E2 : 7 — aucune acceptation indépendante, aucun enrôlement
  humain réel, aucune séparation UID distincte, aucun confinement egress,
  aucune clôture E3, inventaire I15/ACS en attente, effets SQLite fictifs.
- Bloqueurs humain/OS : enrôlement réel (commanditaire, IA3), newuidmap/newgidmap,
  évaluateur E3 distinct, arbitrage superviseur distinct.

## Campagnes T01–T22

| Test | Invariant(s) | État local | Dernière preuve (dossier, date) | Bloqueur restant |
|---|---|---|---|---|
| T01 | I01 | PASS | `paea-delivered-chain-cli-rust` + `paea-delivered-settlement-cli-rust` (ELF dev.267, 31-10, via E2) | Évaluateur externe |
| T02 | I01/I03 | partielle | `glm-native-profile-20261029T181332Z` (provenance de build 713/713) | Flux d'exécution (raccord Mémoire PA-05) |
| T03 | I02 | bloquée | revocation-check 2789aff3… (fixture) | Enrôlement humain réel (IA3) |
| T04 | I04 | PASS | `paea-approval-form-rust.json` / `qualif-20261031T103528Z` (navigateur réel, rejeu périmé refusé 403) | Approbateur humain réel (PA-08) |
| T05 | I05/I16 | PASS | recette révocation, deux ordres (PILOTE-OPERATIONNEL) | Course réelle à l'engagement (production) |
| T06 | I08/I09 | PASS | recette crash post-publication, unicité grand livre (PILOTE-OPERATIONNEL) | — (campagne E3 restante) |
| T07 | I06 | partielle | barrière BT02 incomplète (matrice, 29-10) | Confinement OS |
| T08 | I07 | bloquée | fixture uniquement | Secrets réels / prérequis OS |
| T09 | I08 | PASS | recette `paea-delivered-settlement-cli-rust` / `i17-run-13` (fractionnement dette + re-clé transport refusés, ELF 186cc38b…, 02-11) | Campagne complète non jouée |
| T10 | I10/I12 | bloquée | fixture uniquement | Supervision humaine réelle |
| T11 | I04/I05/I11/I15 | PASS | `paea-i15-tool-substitution-rust.json` / `claude-i15-readmission-20261031T100227Z` (mutation → réadmission prouvée produit) | Campagne complète non jouée |
| T12 | I11 | bloquée | — | Évaluateur indépendant E3 (0/17) |
| T13 | I13 | bloquée | — | Confinement egress absent |
| T14 | I02/I14 | conception manquante | — | Design A2A puis code |
| T15 | I15 | PASS | même preuve que T11 (admission + substitution refusée) | Inventaire ACS (entrées externes) |
| T16 | I12/I16 | PASS | `paea-i16-lease-reopen-rust.json` (30-10, bloc 9) + contrat 8 cas (glm-order-receipt-20261030T075024Z) | Limites distribuées séparées |
| T17 | I05/I07/I17 | PASS | `paea-delivered-settlement-cli-rust` / `i17-run-13` (ELF 186cc38b…, 02-11 midi, 7/7 VERIFIED, étape I17 révocation comprise ; run-12/cdde554a, run-11/41f00bf9 et antérieurs conservés) ; **re-vérifié sur livré courant e2f0039a… (02-11 soir) : `settlement-run-15` 7/7 + `chain-run-8` 4/4 VERIFIED sur worktree gelé 562953dba3** | Propagation production |

Re-vérification du livré courant dev.316/e2f0039a : **complète** (02-11 soir —
run-15 settlement 7/7 + chain-run-8 4/4 VERIFIED sur worktree gelé
562953dba3 ; l'arbre vivant restait incompilable par autrui, voir extension
soirée ci-dessous).
| T18 | I03/I14 | non exercée | — | Périmètre E3 uniquement |
| T19 | I06/I13 | non exercée | — | Périmètre E3 + confinement I06/I13 |
| T20 | I04/I15 | non exercée | — | Périmètre E3 uniquement |
| T21 | I09/I11/I17 | partielle | variante chaos déterministe locale (recette T06/T21, PILOTE-OPERATIONNEL) | Campagne E3 complète |
| T22 | I10 | non exercée | — | Périmètre E3 + humain |

## Pourcentages (numérateurs observés dans les registres)

- Campagnes formelles E3 exécutées : 0/22 (0 %)
- Recette locale PASS : 9/22 (41 %) — T01, T04, T05, T06, T09, T11, T15, T16, T17
- Partielle : 3/22 (14 %) — T02, T07, T21
- Bloquée humain/OS : 6/22 (27 %) — T03, T08, T10, T12, T13, T22
- Non exercée E3 : 3/22 (14 %) — T18, T19, T20
- Conception manquante : 1/22 (5 %) — T14
- Invariants en vérification locale partielle : 10/17 (59 %) — I01, I04, I05,
  I08, I09, I12, I16, I17 + partiels I03, I15
- Acceptation indépendante : 0/17 (0 %)

## Évolution depuis le dernier point

- Point précédent : DASHBOARD-T20261031.md (31 octobre 2026)
- Delta 31-10 → 01-11 : T17 (I17) passe de « partielle » à PASS local sur le
  livré courant — la recette `paea-delivered-settlement-cli-rust` (dont l'étape
  I17 « révocation du courant → refus sans effet puis récupération ») est
  re-jouée 6/6 VERIFIED sur l'ELF dev.292/4164953c… (`.swarm-fixtures/i17-rerun/
  i17-run-4/`). Les pourcentages par état ne changent pas (T17 était déjà
  comptée PASS pour la recette promote ; c'est la portée de preuve qui s'étend
  au chemin settlement-call et au livré courant). Candidat dev.267/1d3f9da1…
  → dev.292/4164953c… (la session parallèle a livré l'incrément « authority
  sessions, signed assertions and engine access mode », commit 8cb059831).
- Incidents de reprise documentés (3 échecs avant le PASS, causes distinctes) :
  run-1 témoin de binaire (remplacement concurrent du livré en cours de run),
  run-2 arbre non compilable (incrément v52 parallèle), run-3 décalage
  binaire/arbre (ELF parser v51, harnais v52 → `settings-format`) ;
  `.swarm-fixtures/i17-rerun/i17-run-{1,2,3}/` conservés.
- Extension 01-11 après-midi : candidat dev.292/4164953c… →
  dev.298/f5a154da… (commit d4e219eea « feat(blp): typed HTTP status
  translation for preparation refusals », construit 13:36Z). Les deux recettes
  delivered re-pinnées et re-jouées sur ce livré : settlement 6/6 VERIFIED
  (`i17-run-5/`, témoins `delivered-binary-witness` à chaque spawn) et chaîne
  4/4 VERIFIED (`chain-run-2/`, refus de SHA erroné compris),
  `source_stable: true`. Deux refus préalables du runner (invocation incomplète,
  sortie non neuve) corrigés sans perte de preuve. T17 reste PASS ; portée
  inchangée, preuve étendue au livré courant.
- Extension 01-11 soir : candidat dev.298/f5a154da… → dev.308/e54a1dd5…
  (commit 0eff289d1f « phase B admin, audit, API keys », binaire 19:09Z) →
  c12004bf… (4e remplacement du jour, 19:44:34Z, en pleine étape 3 du run-7).
  run-6 FAIL (E0433 `AtomicStorageFault` — édition concurrente de
  `preparation_admission.rs` à 19:19:43Z pendant la compilation,
  `source_stable: false`) ; run-7 FAIL à l'étape 3 (le contrôle témoin du test
  a refusé la divergence épingle/réel `e54a1dd5`/`c12004bf`,
  `source_stable: true`) ; `.swarm-fixtures/i17-rerun/i17-run-{6,7}/` conservés.
  Aucune régression produit : à chaque fois le livré a changé sous le run.
- Reprise vérifiée (01-11 soir, porte de stabilité binaire 10 min) : les deux
  recettes re-pinnées c12004bf… et re-jouées — settlement 6/6 VERIFIED
  (`i17-run-8/`, `source_stable: true`, témoins = livré courant) et chaîne 4/4
  VERIFIED (`chain-run-3/`, refus de SHA erroné compris). T17 reste PASS,
  preuve étendue au livré courant ; run-6/run-7 restent conservés comme preuves
  des deux incidents de reprise.
- Extension 01-11 fin de soirée : c12004bf… → 4c47b7e8… (5e swap, ~20:15Z)
  puis 280d01995c52475c5aaebe7588165cc0089a892f88e367c19dd2648a6f66011c
  (6e swap, 21:34:15Z, porte de stabilité adaptative verte 21:34→21:45).
  run-9 FAIL à l'étape 1 : épingle posée avec un hash tronqué/faux lors de la
  re-épingle manuelle (préfixe 16 hex exact, suffixe erroné) — le contrôle
  `grep` de la session comptait les occurrences du hash faux sans le comparer
  au réel ; le témoin interne du test a refusé la divergence
  (`i17-run-9/` conservé, `source_stable: true`). Correction au contrat :
  re-épingle par substitution depuis `sha256sum` réel + cross-check
  épine=real, puis run-10 settlement 6/6 VERIFIED et chain-run-4 4/4 VERIFIED
  (témoins = livré courant dans les stdout, refus de SHA erroné compris),
  `source_stable: true`. T17 reste PASS, preuve étendue au livré courant.
- Extension 02-11 matin : 280d0199… → 41f00bf96e568caaa0ef40cc26b758139acbb
  2133cddb4829dd0965dfa2ad061 (7e remplacement du jour, porte de stabilité
  verte 09:31→09:41). Re-épingle depuis le hash réel `sha256sum` (cross-check
  épine=réel) puis run-11 settlement 6/6 VERIFIED et chain-run-5 4/4 VERIFIED
  (témoins = livré courant, refus de SHA erroné compris), `source_stable: true`.
  T17 reste PASS, preuve étendue au livré courant.
- Extension 02-11 fin de matinée : 41f00bf9… → cdde554aaa55420d7f1739cd4f
  f5052071f9c3dc6a51992c5a8c6c187659872d (8e remplacement en ~27 h, livré
  ~10:36, détecté 10:54, porte de stabilité verte 10:44→10:54). Re-épingle
  depuis le hash réel `sha256sum` (cross-check épine=réel, 10/10 tokens) puis
  run-12 settlement 6/6 VERIFIED et chain-run-6 4/4 VERIFIED (témoins = livré
  courant, refus de SHA erroné compris), `source_stable: true`. T17 reste
  PASS, preuve étendue au livré courant.
- Extension 02-11 midi — T09 passe de « partielle » à PASS local : nouveau
  scénario de fractionnement/cumul (incrément 121) ajouté à la recette
  settlement (7e étape) — scission de la dette provisionnée (210000/420000)
  refusée `paea-settlement-debt-incompatible` avant règlement,
  `paea-settlement-business-intent-conflict` après règlement, re-clé
  transport (`request_id` seul modifié) en replay `replayed=true` sans second
  effet. Premier run FAIL étape 1 : le livré a encore changé sous le run
  (cdde554a… → 186cc38b…, 9e remplacement, dev.315 de mon commit 5c9ca0fe8,
  détecté par le témoin interne) ; re-épingle depuis le hash réel
  `sha256sum` (cross-check épine=réel, 8/8 occurrences) puis run-13 : 7/7
  étapes VERIFIED sur ELF 186cc38b… (`.swarm-fixtures/i17-rerun/i17-run-13/`,
  `source_stable: true`). Le livré courant porte maintenant le scénario
  fractionnement/cumul ; l'ancien run-13 FAIL a été supprimé (recréé net).
- Extension 02-11 après-midi — complétion chaîne sur le livré courant :
  la recette chaîne, restée épinglée sur cdde554a…, est re-épinglée sur
  186cc38b… depuis le hash réel `sha256sum` (cross-check épine=réel, 3/3
  tokens) puis chain-run-7 : 4/4 VERIFIED
  (`.swarm-fixtures/i17-rerun/chain-run-7/`, témoins = livré courant, refus de
  SHA erroné compris, `source_stable: true`). Le livré courant est désormais
  intégralement re-vérifié (settlement run-13 7/7 + chaîne chain-run-7 4/4).
- Extension 02-11 soirée — 10e remplacement : 186cc38b… → e2f0039a1f3a349
  57c161e770c4e5efdd46f4e3a16fc82fe9cf5d9761dc94da3 (v0.1.1-dev.316, HEAD
  562953dba3, construit 19:35:26Z, posé 22:10:33, porte de stabilité verte —
  3 sondes sha256 sur ~3 min, même mtime). Recettes settlement + chaîne
  re-épinglées depuis le hash réel `sha256sum` (cross-check épine=réel,
  8/8 puis 3/3 occurrences). Premier run settlement FAIL fail-closed à
  l'étape 1 (`i17-rerun/settlement-run-14/`, conservé) : compilation du
  harnais impossible sur l'arbre vivant — le diff non commité d'une session
  concurrente (`rust/crates/swarm-authority/src/audit.rs`, +178, enrichissement
  `Event` ADM-05/ADM-10) casse le consommateur commité
  `swarm-web/src/authority_ledger.rs:138` (E0063, 5 champs manquants) ;
  l'enrichissement ni sa mise à jour consommateur ne sont miens à corriger
  (portée d'autrui). Reprise autorisée par la méthode des sources gelées
  (précédent LIVE-UX-01) : worktree détaché à HEAD 562953dba3 — exactement le
  commit d'où le livré est construit — avec l'ELF e2f0039a copié en `bin/swarm`
  (mode 500) ; le témoin sha256 du harnais re-prouve le livré à chaque spawn
  comme sur l'arbre vivant. Settlement re-lancé sur ce gel (run-15) et chaîne
  ensuite ; preuves dans `.swarm-fixtures/i17-rerun/`.
- Extension 02-11 soirée (complétion settlement) — run-15 sur le worktree
  gelé 562953dba3 : **7/7 étapes PASS** sur l'ELF livré e2f0039a…,
  `source_stable: true`, `recipe.py verify` → VERIFIED
  (`.swarm-fixtures/i17-rerun/settlement-run-15/`), 23 spawns témoins
  `sha256=e2f0039a…` re-prouvés (compilation du harnais 1 min 26 s sur
  sources gelées, là où l'arbre vivant échouait E0063). La chaîne
  (`chain-run-8`) suit sur le même gel.
- Extension 02-11 soir (complétion chaîne) — chain-run-8 sur le même worktree
  gelé : **4/4 étapes VERIFIED** sur l'ELF livré e2f0039a…,
  `source_stable: true`, refus de SHA erroné exercé (étape 4), témoins
  `sha256=e2f0039a…` re-prouvés aux spawns (`.swarm-fixtures/i17-rerun/
  chain-run-8/`). Premier lancement refusé `recipe-input-pattern-empty` : la
  fixture source déclarée de la chaîne (`claude-tmp-fix-e2e-20261031T173021Z/
  chain-fixture-regen-20261031b`, evidence locale gitignorée, jamais commitée)
  n'existait pas dans le worktree gelé ; copiée byte-identiquement (diff
  sha256 des 655 fichiers = vide) avant la relance. **Le livré courant
  dev.316/e2f0039a est intégralement re-vérifié (settlement 7/7 + chaîne
  4/4)**, comme dev.315 avant lui.
- Extension 02-11 fin de nuit — re-vérification répétée sur l'**arbre vivant
  recompilé** : la session concurrente a posé la mise à jour consommateur
  (`authority_ledger.rs` : 5 nouveaux champs `None` à l'initialiseur `Event`),
  l'arbre vivant recompile (`cargo check -p swarm-web` et `-p swarm-cli
  --tests` sans erreur). Settlement `settlement-run-16` 7/7 PASS VERIFIED
  `source_stable: true` (23 spawns témoins e2f0039a…), puis chaîne
  `chain-run-9` 4/4 PASS VERIFIED `source_stable: true`, refus de SHA erroné
  compris — toutes deux sur l'arbre vivant, sans gel. Le livré courant porte
  donc une preuve sur chacune des deux surfaces : gel 562953dba3 (run-15 +
  chain-run-8) et arbre vivant (run-16 + chain-run-9).

## Portée et limites

Chaque PASS est portée fixture, auth simulée, same-UID : aucune ne vaut
acceptation indépendante. Lignes bloquées = prérequis humain/OS réels hors
pouvoir des agents. Les pourcentages mesurent l'état local observable, pas une
avancement vers l'acceptation — « les nombres de tests/incréments ne sont pas
un pourcentage d'avancement » (invariant-matrix.json, global_limits).

## Sources

- `docs/plans/actions-confiance/PAEA-APEX.md` — bloc « État courant », matrice,
  section T18–T22
- `paea/evidence/20261028-acquisition81-mvfdts57/glm-closure-inventory-20261029T201336Z/invariant-matrix.json`
  (generated_utc 2026-10-29T20:31:00Z — état détaillé par invariant)
- `.swarm-fixtures/i17-rerun/i17-run-4/` — recette `paea-delivered-settlement-cli-rust`
  6/6 VERIFIED sur ELF dev.292/4164953c… (01-11-2026, 11:05Z), témoin
  `delivered-binary-witness` à chaque spawn
- `.swarm-fixtures/i17-rerun/i17-run-5/` — même recette re-pinnée, 6/6 VERIFIED
  sur ELF dev.298/f5a154da… (01-11-2026, 15:03Z), `source_stable: true`
- `.swarm-fixtures/i17-rerun/chain-run-2/` — recette `paea-delivered-chain-cli-rust`
  re-pinnée, 4/4 VERIFIED sur ELF dev.298/f5a154da… (01-11-2026, 15:06Z)
- `.swarm-fixtures/i17-rerun/i17-run-6/` — FAIL (E0433 édition concurrente
  pendant compilation, `source_stable: false`, 01-11-2026 19:17Z)
- `.swarm-fixtures/i17-rerun/i17-run-7/` — FAIL étape 3 (témoin interne a
  attrapé le remplacement binaire 19:44:34Z, `source_stable: true`)
- `.swarm-fixtures/i17-rerun/i17-run-8/` — 6/6 VERIFIED sur ELF c12004bf…
  (01-11-2026 19:56Z, `source_stable: true`, témoins livré courant)
- `.swarm-fixtures/i17-rerun/chain-run-3/` — 4/4 VERIFIED sur ELF c12004bf…
  (01-11-2026 19:57Z, refus de SHA erroné compris)
- `.swarm-fixtures/i17-rerun/i17-run-9/` — FAIL étape 1 (épingle tronquée
  posée par la session, témoin interne a refusé, `source_stable: true`,
  01-11-2026 21:59Z)
- `.swarm-fixtures/i17-rerun/i17-run-10/` — 6/6 VERIFIED sur ELF
  280d0199…6011c (01-11-2026 22:07Z, `source_stable: true`)
- `.swarm-fixtures/i17-rerun/chain-run-4/` — 4/4 VERIFIED sur ELF
  280d0199…6011c (01-11-2026 22:10Z, refus de SHA erroné compris)
- `.swarm-fixtures/i17-rerun/i17-run-11/` — 6/6 VERIFIED sur ELF
  41f00bf9…a2d061 (02-11-2026 09:44Z, `source_stable: true`)
- `.swarm-fixtures/i17-rerun/chain-run-5/` — 4/4 VERIFIED sur ELF
  41f00bf9…a2d061 (02-11-2026 09:45Z, refus de SHA erroné compris)
- `.swarm-fixtures/i17-rerun/i17-run-12/` — 6/6 VERIFIED sur ELF
  cdde554a…9872d (02-11-2026 ~10:55Z, `source_stable: true`)
- `.swarm-fixtures/i17-rerun/chain-run-6/` — 4/4 VERIFIED sur ELF
  cdde554a…9872d (02-11-2026 ~10:57Z, refus de SHA erroné compris)
- `.swarm-fixtures/i17-rerun/i17-run-13/` — recette settlement étendue à 7
  étapes, 7/7 VERIFIED sur ELF 186cc38b…7853 (dev.315, 02-11-2026 midi,
  `source_stable: true`) — nouvelle étape T09 fractionnement/cumul (run
  précédent FAIL sur remplacement binaire concurrent, supprimé net)
- `.swarm-fixtures/i17-rerun/chain-run-7/` — 4/4 VERIFIED sur ELF
  186cc38b…7853 (02-11-2026 après-midi, refus de SHA erroné compris,
  `source_stable: true`) — complétion chaîne du livré courant
- `paea/DASHBOARD-T20261031.md` — instantané précédent (31 octobre 2026)
- `paea/PILOTE-OPERATIONNEL.md` — recettes initiales T04, T06/T21, T05/T17
- `paea/E3-PROTOCOL.md` — périmètre des campagnes formelles
