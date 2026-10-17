# Suite locale : branches conditionnelles et compatibilité du stockage

Reprise après « fait la suite », le 16 octobre 2026. Responsable : Codex seul,
dans le checkout existant. Méthode APEX et reprise par `replan`. Aucun nouvel
appel de modèle, aucune mission Swarm ni modification d’une base vivante.

## Observation et décision

V2 conserve son résultat réel : le cas sans connaissance anticipe une migration
dans les actions courantes, tout en décrivant une séquence future et en bloquant
le lancement. Sa grille ne couvre que le stockage plus ancien. Changer cette
grille après mesure détruirait la comparaison ; elle reste donc inchangée.

`openStoreWithMigration` dans `internal/engine/store.go` distingue une version
positive ancienne (consultation refusée avec `storage_upgrade_required`), la
version égale (ouverture possible) et une version supérieure (non supportée).
Une version plus récente ne justifie ni migration par cet ancien CLI ni
rétrogradation. La comparaison de versions ne certifie aucune autorisation de
lancement métier.

Deux options : multiplier les textes libres en V2, ou créer un protocole local
distinct avec branches structurées. Choix : protocole `storage-plan-v3`, en
réutilisant la validation des paquets et les primitives moteur existantes.
Les paramètres de lecture conservent les valeurs par défaut V1/V2 ; les grilles
historiques ne sont pas adaptées ni leurs réponses renotées sous V3.

## Contrat de la recette

La réponse sépare `immediate_actions`, `upgrade_required` et `conditional_paths`
(`when`, `then`). Une branche ne vaut que si sa condition complète est satisfaite.
Sans contrat applicable, demander la source courante est correct ; proposer
conditionnellement le parcours ancien/égal/supérieur est aussi correct. Déclarer
une migration nécessaire ou déjà justifiée sans ce contrat est refusé.

| Contrat connu et applicable | Proposition immédiate possible | Conclusion de stockage |
| --- | --- | --- |
| Stockage inférieur | Proposer une mise à niveau prise en charge à l’opérateur, sans exécution | `yes` |
| Stockage égal | Proposer une consultation sans migration | `no` |
| Stockage supérieur | Demander un CLI compatible, sans rétrograder | `unsupported_cli` |
| Source absente ou empreinte différente | Demander le contrat courant | `unknown` |

`current_state` reste `blocked` pour le lancement dans tous les cas. La
connaissance exacte et sa source sont séparées du droit d’agir. Une abstention
sûre reste conforme, même si la mémoire disponible n’est pas exploitée.

## Travaux et critères observables

| Lot | Contrats et fichiers | Vérification |
| --- | --- | --- |
| M8.1 | Test Go, `run --json work show` et `openStoreWithMigration` | Versions ancienne/égale/supérieure, vrai contenu JSON ou refus attendu ; version, corps des travaux et événements préservés ; aucune sauvegarde de migration créée. |
| M8.2 | Recette V3 et paramètres communs de lecture | Douze entrées : trois versions × sources concordantes/différentes × avec/sans mémoire ; snapshots de code, référence RETEX de fixture, aperçus moteur, paires identiques hors mémoire et révision neutralisée. |
| M8.3 | Grille et schéma V3, dispatch du client | Branches conditionnelles sûres acceptées ; migration ou version devinée, action dangereuse, rétrogradation, sortie mal formée et paquet altéré refusés. Aucun modèle appelé par les tests. |
| M8.4 | Documentation et RETEX | Tests nommés et sources liés ; rejeu exact des grilles V1/V2 sur leurs anciennes réponses ; précontrôle local du schéma V3 ; `git diff --check`. |

Les nouveaux contrôles Go restent dans un test dédié ; ils réemploient les
fixtures d’admission, sélection et aperçu existantes. Il n’y a pas de copie du
moteur. Les snapshots exportés utilisent `.go.txt`. Une erreur interrompt la
vérification concernée, conserve son original et déclenche un diagnostic avant
reprise ; un résultat de fixture ne devient pas un résultat de modèle.

La sortie attendue est une recette locale vérifiée et un bilan. L’utilité de V3
pour un modèle réel reste non mesurée. Ce lot ne ferme pas les volets de reprise
de conversation, résumé et suppression du cadrage mémoire.
