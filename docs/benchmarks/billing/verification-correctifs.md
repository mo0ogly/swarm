# Vérification des correctifs du moteur (pré-enregistrement)

Statut : protocole du 7 octobre 2026, rédigé **avant** la campagne de vérification. Les
correctifs sont ceux du commit `53752da` (branche `codex/billing-engine-recovery`, documentés
dans `docs/en/ENGINE-BUSINESS-CONTROLS.md`). Le binaire mesuré est construit depuis ce commit,
dans un arbre de travail sans modification. Les résultats historiques (`campagne-20261005.jsonl`)
ne sont ni recalculés ni remplacés.

## Ce qui change dans le banc

- Les contrôles déclarent `environment_exit_codes: [3]` (correctif D3) et le travail déclare
  `requirement_prerequisites` : `req-2` attend `req-1` (et `req-3` sous F4e) (correctif D6).
  Ces deux réglages viennent de la branche des correctifs.
- Trois variantes sans faute, clé métier :
  - `nodeps` : `settle` est créée **sans** dépendance déclarée ; la règle de prérequis est active ;
  - `noguard` : ni dépendance ni règle ; **contrôle positif** de D6 ;
  - `ownws` : chaque tâche a son propre espace de tentative, sans espace partagé (D1).
  Les variantes D6 utilisent aussi des espaces propres : l'espace partagé sérialise déjà les
  tâches et masquerait l'effet de la règle (constaté en mise au point : sans espace propre,
  `noguard` attendait quand même).
- Chaque exécution consigne : tentatives de `prepare`, reçu d'échec d'environnement, nombre
  d'événements `dependency_stale`, départ de `settle` avant ou après l'acceptation de `prepare`.

## Grille

Condition S, agents scriptés, une exécution par case et par graine (1000 à 1099), en série :

| Bloc | Cases | Exécutions |
|---|---|---|
| D5, D2 | F4e × clés aucune, tentative, métier | 300 |
| D3 | F5 × clés aucune, tentative, métier | 300 |
| D6, D1 | variantes nodeps, noguard, ownws × clé métier | 300 |
| Non-régression | clé métier × aucune, F1, F2, F3, F4, F6, F7, F8 | 800 |

Total : 1 700 exécutions, environ 10 heures, aucun appel de modèle.

## Critères (figés dans `bench/tables_verif.py`)

| Défaut | Corrigé si | Historique |
|---|---|---|
| D5 | aucune exclusion due à SQLITE_BUSY sous F4e | 31 sur 300 |
| D3 | sous F5, toutes les exécutions valides : `prepare` lancée une fois, reçu d'échec d'environnement, ni faux succès ni paiement inexact | 2 tentatives, régénération du lot |
| D2 | sous F4e, toutes les exécutions valides : au moins un `dependency_stale` et arrêt attribué au moteur | arrêt muet |
| D6 | `nodeps` : aucun départ de `settle` avant l'acceptation de `prepare`, règlement exact ; et contrôle positif `noguard` : au moins un départ trop tôt (ou ERREUR code 5 du règlement) | non exercé |
| D1 | `ownws` : toutes les exécutions exactes | tâche jamais acceptée |
| Non-régression | clé métier : pour chaque cas et chaque mesure, défaut présent ou absent comme dans la campagne historique | — |

Un critère non tenu est publié comme tel (« non corrigé ») ; aucun critère n'est assoupli après
lecture des données. Le contrôle positif `noguard` produit des ERREUR par construction : elles
sont attendues et ne comptent pas comme exclusions des autres blocs.

## Ce que cette campagne ne vérifie pas

- D3 : la revalidation explicite après rétablissement (`validation recheck-preview/apply`) n'est
  pas exercée ; la campagne vérifie seulement l'absence de régénération.
- D6 : la règle est déclarée par l'opérateur ; un responsable réel qui créerait des tâches sans
  exigence n'est pas simulé.
- Les agents restent scriptés.
