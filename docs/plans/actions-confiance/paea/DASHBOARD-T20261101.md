# Tableau de bord PAEA — 1er novembre 2026

Instantané d'état construit depuis les registres réels par `/apex`. Sources
complètes en fin de fichier. Rappel : ce fichier est un instantané sourcé,
pas une revendication d'acceptation ; les cas locaux de PA-01 ne sont pas les
campagnes T01…T22 exécutées.

## Candidat et blocage courant

- Binaire désigné : `bin/swarm` SHA-256
  `4164953c841c2b22476bd552a12496aac2b7f95e9b26de04d986c3238e2ed83c`
  (v0.1.1-dev.292, commit 8cb05983175bfe6d4195337503aa75e39b4865a5 « authority
  sessions, signed assertions and engine access mode », construit
  2026-11-01T09:57:50Z, `modified=true` — arbre sale au build)
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
| T09 | I08 | partielle | grand livre/ref Git local | Fractionnement/cumul multi-agents complet |
| T10 | I10/I12 | bloquée | fixture uniquement | Supervision humaine réelle |
| T11 | I04/I05/I11/I15 | PASS | `paea-i15-tool-substitution-rust.json` / `claude-i15-readmission-20261031T100227Z` (mutation → réadmission prouvée produit) | Campagne complète non jouée |
| T12 | I11 | bloquée | — | Évaluateur indépendant E3 (0/17) |
| T13 | I13 | bloquée | — | Confinement egress absent |
| T14 | I02/I14 | conception manquante | — | Design A2A puis code |
| T15 | I15 | PASS | même preuve que T11 (admission + substitution refusée) | Inventaire ACS (entrées externes) |
| T16 | I12/I16 | PASS | `paea-i16-lease-reopen-rust.json` (30-10, bloc 9) + contrat 8 cas (glm-order-receipt-20261030T075024Z) | Limites distribuées séparées |
| T17 | I05/I07/I17 | PASS | `paea-delivered-settlement-cli-rust` / `i17-run-8` (ELF c12004bf…, 01-11 soir, 6/6 VERIFIED — révocation settlement-call refusée puis récupération ; run-5/dev.298 et run-4/dev.292 conservés) | Propagation production |
| T18 | I03/I14 | non exercée | — | Périmètre E3 uniquement |
| T19 | I06/I13 | non exercée | — | Périmètre E3 + confinement I06/I13 |
| T20 | I04/I15 | non exercée | — | Périmètre E3 uniquement |
| T21 | I09/I11/I17 | partielle | variante chaos déterministe locale (recette T06/T21, PILOTE-OPERATIONNEL) | Campagne E3 complète |
| T22 | I10 | non exercée | — | Périmètre E3 + humain |

## Pourcentages (numérateurs observés dans les registres)

- Campagnes formelles E3 exécutées : 0/22 (0 %)
- Recette locale PASS : 8/22 (36 %) — T01, T04, T05, T06, T11, T15, T16, T17
- Partielle : 4/22 (18 %) — T02, T07, T09, T21
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
- `paea/DASHBOARD-T20261031.md` — instantané précédent (31 octobre 2026)
- `paea/PILOTE-OPERATIONNEL.md` — recettes initiales T04, T06/T21, T05/T17
- `paea/E3-PROTOCOL.md` — périmètre des campagnes formelles
