# P0 — Contrat Cursor et écarts explicites

Date de la tentative : 19 septembre 2026  
Périmètre : architecture de responsabilités et flux de retour uniquement.  
Source comparée : [Cursor, « Towards self-driving codebases », section « The final system design »](https://cursor.com/blog/self-driving-codebases#the-final-system-design), consultée le 19 septembre 2026.

## Verdict borné

Le contrat de responsabilités P0 est implémenté et couvert sur le chemin de planification hiérarchique : le responsable est sans outils d’écriture, les sous-périmètres délèguent récursivement dans des limites explicites, les tâches exécutables sont exclusivement des `worker`, les échanges latéraux sont refusés, une remise structurée remonte au propriétaire du périmètre, puis ce propriétaire peut adapter une reprise bornée.

Ce verdict ne signifie pas « parité complète avec Cursor » ni « autonomie démontrée ». Les écarts et les éléments non démontrés sont listés ci-dessous. La revue IA indépendante reste une extension Swarm et n’est pas attribuée à l’architecture finale de Cursor.

## Décisions et changements

1. **Responsables non exécutables.** `planner` et `subplanner` désignent maintenant des `PlanningScope`, pas des tâches lançables. Le contrat du plan de préparation n’accepte que `role: "worker"`. La validation d’une organisation importée refuse un rôle exécutable `planner`/`subplanner`. Une ancienne tâche hiérarchique sans rôle explicite reste interprétée comme worker pour compatibilité ; un rôle explicitement non-worker est refusé.
2. **Séparation technique, pas seulement textuelle.** Le responsable passe par `assistantProvider` : Codex est en bac à sable lecture seule, les outils shell et multi-agent sont désactivés, la configuration utilisateur est ignorée ; Claude reçoit une liste d’outils vide et une configuration MCP vide. Le lancement d’une tâche hiérarchique force le rôle worker.
3. **Isolement des exécutants.** Le chemin hiérarchique refusait déjà `exchange send`; le contrôle est maintenant complété au lancement par le refus de `Agent.Parent`. Un exécutant ne peut donc pas former une hiérarchie parallèle ou communiquer directement avec un autre exécutant via le protocole Swarm. Les missions historiques non hiérarchiques conservent leur protocole d’échange existant.
4. **Retour au propriétaire.** Une remise exige l’identité du worker, de la tâche et de la tentative courante, un constat, et un ou plusieurs artefacts vérifiés par empreinte. L’événement est routé vers le `ScopeID` propriétaire. La fin du processus et la validation produisent des événements distincts ; aucune ne vaut acceptation implicite.
5. **Réactivation et adaptation.** Une fin de tentative, une remise, une validation modifiée, la clôture d’un enfant ou une preuve devenue périmée remet le périmètre concerné en état `ready`. Une reprise exige une correction explicite différente et respecte le plafond de tentatives.
6. **Audit exécutable.** `tests/audit_acceptance.cjs --case cursor` lance les tests Go `TestCursorContract*`. Un cas inconnu sort avec le code 2 et une dépendance absente avec le code 3. Le script ne déduit jamais un succès de la présence de ce rapport.

## Correspondance avec le design final de Cursor

| Principe Cursor | État Swarm P0 | Preuve exécutable ou code | Limite |
| --- | --- | --- | --- |
| Responsable racine propriétaire du besoin | Implémenté | `PlanningScope{id:"root"}` reçoit l’objectif et toutes les exigences ; `TestCursorContractPlannerIsToolFreeAndRootOwnsNeed` | La propriété est bornée aux exigences enregistrées par l’opérateur. |
| Le responsable ne code pas | Implémenté sur le chemin hiérarchique | `assistantProvider`, `planningStep`, test des arguments lecture seule et outils désactivés | Une configuration fournisseur inconnue est refusée ; aucun essai fournisseur réel dans cette tentative. |
| Délégation récursive de périmètres | Implémenté, borné | opérations `delegate`, propriété exclusive des exigences, test root → child → leaf | Profondeur maximale 3 et 20 périmètres : écart volontaire de sûreté et de coût. |
| Exécutants aveugles au système global | Implémenté par projection | prompt limité au périmètre/tâche ; contexte des responsables projeté par scope | Un worker voit encore le brief et les pièces explicitement autorisées de sa tâche. |
| Pas de communication directe entre exécutants | Implémenté pour les missions hiérarchiques | `sendExchangeOnce` refuse toute mission avec `Planning`; lancement avec `Parent` refusé ; test de cross-talk | Les missions historiques non hiérarchiques gardent les échanges directs autorisés par leur plan. |
| Copie de dépôt propre au worker | Implémenté seulement en mode dépôt Git géré | `ensureManagedAttempt`, tests `TestManaged*` | La préparation standard peut utiliser un espace partagé sérialisé : ce n’est pas présenté comme équivalent au modèle Cursor. |
| Remise unique structurée au responsable demandeur | Implémenté | `planning handoff`, identité de tentative, artefacts SHA-256, déduplication ; test du scope feuille | La fin de processus ajoute aussi un événement de cycle de vie distinct ; ce n’est pas une seconde remise de résultat. |
| Réactivation après retour et adaptation du plan | Implémenté | scope `ready`, événements `attempt_ended`, `handoff`, `validation_changed`, `scope_closed`, `proof_stale`; test d’une reprise avec correction nouvelle | Bornes d’activations, de décisions et de tentatives ; pas de boucle infinie. |

## Écarts volontaires

### Revue indépendante

Cursor décrit un juge dans une version intermédiaire puis indique l’avoir retiré de son système final. Swarm conserve un vérificateur IA séparé, sans outils, comme extension de contrôle. Son avis ne remplace ni les contrôles déterministes, ni l’acceptation humaine lorsqu’elle est requise. Cette extension ne doit donc pas être citée comme une exigence Cursor.

### Intégration centrale

Cursor a retiré son intégrateur central parce qu’il devenait un goulot à très grande échelle. Swarm conserve, en mode Git géré, une intégration centrale sérialisée par verrou : fusion sur le candidat courant, réexécution des contrôles cumulés, vérification d’absence de mutation par les contrôles, puis publication atomique du pointeur candidat.

Le risque est explicite : le débit est limité par la fusion et le jeu de contrôles les plus lents ; une file importante peut donc sérialiser les workers même avec plusieurs créneaux. Le choix est volontaire pour préserver la traçabilité de la révision réellement testée et éviter qu’une fusion concurrente invalide les preuves. Aucun chiffre de débit ou de seuil acceptable n’est démontré ici. Les tests `TestManaged*` prouvent la cohérence et l’attente du verdict, pas la capacité à l’échelle de centaines d’agents.

### Tolérance aux erreurs

Swarm n’adopte pas la tolérance de Cursor consistant à accepter un faible taux d’erreurs pour maximiser les commits par heure. Sur le chemin géré, un conflit, un contrôle en échec, une preuve manquante ou périmée bloque la publication/fermeture. Une correction peut être proposée dans une limite existante, mais un processus terminé n’est jamais assimilé à un résultat accepté. Le compromis est un débit potentiellement inférieur en échange d’un verdict attribuable à une révision et à des contrôles.

### Visibilité du responsable

Cursor indique que le responsable racine n’a pas à savoir si une tâche a été prise ni par qui. Swarm expose au responsable des états de tâche et des identifiants de tentative bornés, sans transcript global. C’est un écart volontaire d’observabilité et de reprise : ces informations permettent d’éviter une reprise sur une tentative obsolète, mais augmentent le contexte par rapport au modèle Cursor.

## Contrôles réellement exécutés

| Commande | Résultat observé | Portée |
| --- | --- | --- |
| `node tests/audit_acceptance.cjs --case cursor` via `npm run test:audit:cursor` | code 0, `ok swarm.local/companion`, message de réussite du cas Cursor | Responsabilités, délégation, isolement, handoff et adaptation. |
| `node tests/audit_acceptance.cjs --case does-not-exist` | code 2, `unknown audit acceptance case` | Échec explicite d’un cas inconnu. |
| Suite Go P0 responsabilités/flux, avec caches sous `/dev/shm` | code 0 | Flux hiérarchiques, organisation, dépôt géré et contrat de lancement. |
| Suite Go plan/préparation, avec caches sous `/dev/shm` | code 0 | Contrat de plan, conversion de préparation et séparation planner/worker. |
| `npm test` | code 0 | Graphe, vue mission, rafraîchissement cockpit et catalogues i18n. |
| `go test ./...` avec caches sous `/dev/shm` | code 1 | Le premier passage a révélé puis permis de corriger une compatibilité `PlanRole` vide. Avec `TMPDIR=/dev/shm`, les tests PTY atteignent ensuite `listen unix` mais le bac à sable refuse `setsockopt`; `httptest` ne peut pas non plus ouvrir son socket TCP (`operation not permitted`). Les suites P0 ciblées ont ensuite passé. |

Le tout premier lancement Go, avant déplacement du cache, n’a exécuté aucun test : le cache par défaut `/home/fpizzi/.cache/go-build` était en lecture seule. Les relances probantes utilisent `GOCACHE`, `TMPDIR` et `GOTMPDIR` sous `/dev/shm`.

Commandes exactes des deux suites Go ciblées :

```sh
go test -count=1 -run '^(TestCursorContract|TestPlanning|TestOrganization|TestManaged|TestA8LaunchContract)' .
go test -count=1 -run '^(TestPlan|TestPreparationConversion|TestPreparationPersists|TestPreparedAPI)' .
```

## Éléments non démontrés

- Aucun fournisseur IA réel n’a été appelé ; l’absence d’outil du responsable est testée sur la configuration produite et avec des fournisseurs factices.
- Aucun débit à grande échelle, absence de goulot, autonomie longue durée ou taux d’erreur acceptable n’est démontré.
- Le mode standard à espace partagé n’offre pas une copie Git par worker ; seule la variante dépôt géré couvre ce point.
- Les tests n’observent pas une équipe humaine ni un utilisateur novice.
- La correction ne démontre pas la qualité métier d’un livrable produit par IA ; elle démontre le routage, les responsabilités et les refus du moteur.
- La suite Go complète n’est pas verte dans ce bac à sable pour les raisons d’environnement consignées ci-dessus. Aucun de ces échecs n’est transformé en succès global.

## Handoff au responsable

Changements à relire : contrat des rôles de plan, garde de lancement hiérarchique, validation d’état importé, tests `TestCursorContract*` et runner d’acceptation P0. Risque principal : les plans qui utilisaient explicitement `planner`/`subplanner` comme tâches exécutables sont désormais refusés au lieu d’être lancés avec des outils ; c’est intentionnel, mais mérite une note de migration si de tels plans existent hors des données de test. Prochaine action recommandée : revue indépendante du diff, puis exécution de la suite Go complète dans un environnement autorisant les sockets Unix et TCP locaux. Ne pas conclure à une autonomie démontrée à partir de ce lot.
