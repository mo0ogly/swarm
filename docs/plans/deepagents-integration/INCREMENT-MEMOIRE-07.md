# État durable aux frontières des tours de dialogue

16 octobre 2026, reprise « suite ». Codex seul applique APEX et `replan`.
Exigences : MEM-U08, MEM-U12, MEM-U13 ; cas MEM-C15 et MEM-C16.

## Observation et choix

M10 qualifie la fin, mais le tour peut être répondu avant le prochain battement
du superviseur. Les usages cumulés, signatures de répétition, erreurs et outils
en attente vivent dans `dialogueMonitor` et `loopGuard`. `agent_logs` ne peut pas
servir de journal de restauration : il est limité à 2 000 entrées et sa capture
de sortie est facultative et tronquée. Réutiliser les seuls compteurs visibles
remettrait à zéro les gardes internes.

Options : reconstruire depuis les journaux incomplets, ajouter un nouveau magasin,
ou conserver un état versionné sur le tour existant. Choix : une colonne
`checkpoint` de `agent_dialogue_turns`, sous clôture de compatibilité du stockage.
La sauvegarde de migration précède le changement. Les copies de cycle de vie
capturent déjà toutes les colonnes de cette table. Aucun autre service ni
politique de rétention ou de seuil n’est ajouté.

L’état est enregistré avant l’autorisation de début et avant la confirmation
de fin. Il lie projet, agent, tentative, tour, session, configuration de lancement,
limites et gardes cumulées, avec usage observé et empreinte. Une interruption au
milieu d’un tour reste incertaine : un état de début ne décrit pas les effets
survenus ensuite. Le statut `answered` et un état confirmé sont tous deux requis
pour qualifier une frontière complète. Aucun rejeu ni nouveau droit de reprise.

## Séquence et preuves attendues

| Lot | Propriétaire et périmètre | Critère |
| --- | --- | --- |
| M11.1 | Codex, reproduction et producteurs réels | Montrer qu’un tour répondu n’a pas encore son usage durable avant le battement ; préserver cette preuve. |
| M11.2 | Codex, état sérialisé et persistance | Sauvegarde avant départ/fin ; restauration technique identique des gardes dans une fixture ; panne d’écriture bloque le fournisseur ou la continuation. |
| M11.3 | Codex, lectures CLI/API et compatibilité | Exposer la frontière et ses limites sans état interne privé ; ancienne absence explicite, corruption refusée ; migration isolée préservant histoire et sauvegarde ; cycle de vie conservé. |
| M11.4 | Codex, vérification et RETEX | Parcours binaire local, tests ciblés/race, analyse statique, docs FR/EN, preuves liées aux sources. |

Les sous-processus fournisseurs sont des scripts locaux. Le serveur déjà lancé
et sa base restent intacts. Les huit évaluations réelles précédentes ne sont ni
relancées ni renotées. La restauration technique de la garde dans un test ne
constitue pas la reprise autorisée d’une conversation : le refus de redémarrage
implicite reste en place.

## Résultat

M11.1 à M11.4 sont réalisés et vérifiés localement. Deux critères échouent avant
réalisation ; un défaut de temps dans la nouvelle projection est ensuite reproduit
et corrigé. Le candidat final passe 32 tests nommés, dont 16 avec `-race`, ainsi
que l’analyse statique et le parcours du binaire construit. Migration et cycle de
vie sont testés dans des racines isolées ; documentation FR/EN et RETEX sont dans
[RESULTATS-MEMOIRE-07.md](RESULTATS-MEMOIRE-07.md). Aucun stockage utilisateur n’a
été migré et aucun redémarrage fournisseur n’est autorisé.
