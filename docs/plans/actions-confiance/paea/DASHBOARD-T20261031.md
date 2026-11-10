# Tableau de bord PAEA — 31 octobre 2026

Instantané d'état construit depuis les registres réels par `/apex`. Sources
complètes en fin de fichier. Rappel : ce fichier est un instantané sourcé,
pas une revendication d'acceptation ; les cas locaux de PA-01 ne sont pas les
campagnes T01…T22 exécutées.

## Candidat et blocage courant

- Binaire désigné : `bin/swarm` SHA-256
  `1d3f9da1408a82b14b5ae5d76328b58c52f1f92051824af52f1d413be8dd41fd`
  (v0.1.1-dev.267, 6706e201b + travail local 31-10, construit 2026-10-31T11:57Z)
- Déclaration E2 bornée : **émise** le 31 octobre 2026
  (`claude-e2-arbitrage-20261031T135859Z/DECLARATION-E2.md`) — règlement 6/6 et
  chaîne 4/4 PASS VERIFIED sur fixture regen-5 ; clôt l'étape PA-10 seulement.
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
| T17 | I05/I07/I17 | PASS | recette grant préémis, ordre inverse joué (PILOTE-OPERATIONNEL) | Propagation production |
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

- Point précédent : POINT-ETAPE-20261029.md (29 octobre 2026)
- Delta 29→31 octobre : +1 invariant localement prouvé (I15 barrière
  mutation→réadmission, bloc 15) ; +1 invariant à preuve navigateur réelle
  (I04 formulaire d'approbation, bloc 14) ; déclaration E2 émise (PA-10 clos,
  bloc 16) ; candidat dev.263/90c3bf90… → dev.267/1d3f9da1….
- Nouveaux dossiers de preuve depuis ce point : `claude-i15-readmission-20261031T100227Z`,
  `glm-form-qualification-20261031T091823Z`, `claude-e2-arbitrage-20261031T135859Z`.

## Portée et limites

Chaque PASS est portée fixture, auth simulée, same-UID : aucune ne vaut
acceptation indépendante. Lignes bloquées = prérequis humain/OS réels hors
pouvoir des agents. Les pourcentages mesurent l'état local observable, pas une
avancement vers l'acceptation — « les nombres de tests/incréments ne sont pas
un pourcentage d'avancement » (invariant-matrix.json, global_limits).

## Sources

- `docs/plans/actions-confiance/PAEA-APEX.md` — bloc « État courant — 31 octobre
  2026 », matrice lignes 143–157, section T18–T22
- `paea/evidence/20261028-acquisition81-mvfdts57/glm-closure-inventory-20261029T201336Z/invariant-matrix.json`
  (generated_utc 2026-10-29T20:31:00Z, ELF 47d31746… — état détaillé par invariant)
- `paea/POINT-ETAPE-20261029.md` — blocs 13–16 (blocs ajoutés 30–31 octobre)
- `paea/PILOTE-OPERATIONNEL.md` — recettes initiales T04, T06/T21, T05/T17
- `paea/E3-PROTOCOL.md` — périmètre des campagnes formelles
