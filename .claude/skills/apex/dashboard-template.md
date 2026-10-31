# Tableau de bord PAEA — <date ISO 8601>

Modèle pour `/apex` : remplir chaque champ depuis les registres réels
(PEUT-APEX « État courant », matrice I01–I17, registre T01–T22 y compris T18–T22
adaptatives, dossier de preuve désigné). Règle : aucun numérateur non observé ;
écrire « non mesuré » plutôt qu'un chiffre inventé. Ce fichier est un instantané
d'état sourcé, pas une revendication d'acceptation.

## Candidat et blocage courant

- Binaire désigné : `<bin/swarm SHA-256>` (<version, commit>)
- Service local : `<PID / « non installé »>` — preuve : `<pointeur>`
- Critères ouverts E2 : `<n>` — `<pointeur DECLARATION-E2>`
- Bloqueurs humain/OS : `<liste courte avec propriétaire>`

## Campagnes T01–T22

| Test | Invariant(s) | État local | Dernière preuve (dossier, date) | Bloqueur restant |
|---|---|---|---|---|
| T01 | I01 | <PASS / partielle / bloquée / non exercée / conception manquante> | <dossier, date> | <cause> |
| T02 | I01/I03 | | | |
| T03 | I02 | | | |
| T04 | I04 | | | |
| T05 | I05/I16 | | | |
| T06 | I08/I09 | | | |
| T07 | I06 | | | |
| T08 | I07 | | | |
| T09 | I08 | | | |
| T10 | I10/I12 | | | |
| T11 | I04/I05/I11/I15 | | | |
| T12 | I11 | | | |
| T13 | I13 | | | |
| T14 | I02/I14 | | | |
| T15 | I15 | | | |
| T16 | I12/I16 | | | |
| T17 | I05/I07/I17 | | | |
| T18 | I03/I14 | | | |
| T19 | I06/I13 | | | |
| T20 | I04/I15 | | | |
| T21 | I09/I11/I17 | | | |
| T22 | I10 | | | |

États permis (colonne « État local ») : `PASS` (recette locale sur binaire
désigné), `partielle` (barrière prouvée, critères restants), `bloquée`
(prérequis humain/OS), `non exercée` (périmètre E3 uniquement), `conception
manquante`. Rappel du plan : les cas locaux de PA-01 ne sont pas les campagnes
T01…T22 exécutées ; les campagnes formelles restent du périmètre E3.

## Pourcentages (numérateurs observés dans les registres)

- Campagnes formelles E3 exécutées : <n>/22 (<p> %)
- Recette locale PASS : <n>/22 (<p> %)
- Partielle : <n>/22 (<p> %) · Bloquée humain/OS : <n>/22 (<p> %)
- Non exercée E3 : <n>/22 (<p> %) · Conception manquante : <n>/22 (<p> %)
- Invariants en vérification locale partielle : <n>/17 (<p> %)
- Acceptation indépendante : <n>/17 (<p> %)

## Évolution depuis le dernier point

- Date du point précédent : `<date>` — delta : <changements observés>
- Nouveaux dossiers de preuve depuis ce point : <liste>

## Portée et limites

<Portée des ✅ : fixture, auth simulée, same-UID ; aucune valeur d'acceptation
indépendante. Limites reprises du dossier de preuve désigné.>

## Sources

- <chemin du plan APEX + lignes>
- <chemin invariant-matrix.json + generated_utc>
- <chemin POINT-ETAPE / dossiers de preuve>
