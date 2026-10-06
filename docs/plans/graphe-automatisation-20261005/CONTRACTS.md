# Contrats proposés pour le plan graphique et les déclencheurs

Statut : spécification proposée. Les noms ci-dessous ne sont pas des commandes ou routes existantes garanties. A02 doit les adapter aux opérations publiques déjà exposées. Ne pas documenter ces exemples comme immédiatement exécutables.

## Autorité et couches

Service Go commun → garde et transaction → état et événement durable → projection. HTTP et CLI appellent ce même service. Le canvas prépare des propositions et rend les réponses ; une modification locale du DOM n'est pas une révision du plan.

L'affichage conserve un état séparé : orientation, zoom, viewport, sélection, positions et repli. S'il est synchronisé entre appareils, sa persistance est une opération de présentation sans conséquence sur la fraîcheur des preuves.

## Exemple de prévisualisation

```json
{
  "schema_version": 1,
  "work_id": "exemple-mission",
  "expected_revision": 12,
  "draft_id": "exemple-brouillon",
  "operations": [
    {"kind": "add_dependency", "prerequisite": "t1", "dependent": "t2"}
  ]
}
```

Réponse attendue : digest de proposition, base revision, token de preview, opérations normalisées, permissions évaluées, impact sur descendants, preuves affectées, erreurs structurées et warnings. La prévisualisation seule ne consomme aucune tentative d'agent et ne lance rien. La quantité d'opérations, taille de payload et profondeur sont bornées par configuration explicite ; pas de valeur cachée nouvelle dans ce document.

Application : événement stable + token + contenu et base liée. Revalidation transactionnelle des IDs, cycles, rôles, tâches actives et impacts. Même événement et même contenu → réponse idempotente. Conflit → aucune application partielle ; le client recharge, recompose et prévisualise avant nouvelle soumission.

## Erreurs stables proposées

| Code | Situation | Action utilisateur |
| --- | --- | --- |
| invalid_input | Schéma ou champ non admis | Corriger le champ indiqué |
| unknown_task | Extrémité absente | Recharger le plan et choisir la tâche |
| dependency_cycle | Chemin circulaire | Montrer le cycle et retirer un lien proposé |
| duplicate_dependency | Relation déjà présente | Réutiliser le lien existant |
| revision_conflict | Base modifiée | Recharger et prévisualiser les différences |
| preview_stale | Jeton ne correspond plus aux entrées | Demander une nouvelle prévisualisation |
| active_scope_conflict | Modification touche un périmètre actif | Examiner la portée ; attendre ou demander pause admissible |
| authorization_required | Action hors autorisation | Afficher qui peut décider et la portée nécessaire |
| idempotency_conflict | Même identité et contenu différent | Consulter l'opération originale, corriger la demande |
| provider_unavailable | Capacité/quota non disponible | Conserver le motif et les délais ; vérifier avant reprise |
| workspace_wait | Espace réservé | Attendre sa libération ; pas de nouveau producteur |
| terminal_target | Mission déjà clôturée | Consulter son résultat ; préparer une autre mission uniquement par choix explicite |
| uncertain_effect | Effet externe non confirmé | Vérifier l'état lié à la même demande |

Codes à aligner avec CommandError existant ; HTTP et CLI doivent rendre le même motif métier. Les messages FR/EN ne servent pas d'identifiant de programme. Le mapping précis vers statuts HTTP et exit codes est figé en A02 et testé, sans collisions entre succès et attente.

## Exemple de déclencheur

```json
{
  "schema_version": 1,
  "name": "Reprendre la mission le matin",
  "target_work_id": "exemple-mission",
  "action": "request_resume",
  "timezone": "Europe/Paris",
  "schedule": {"kind": "once", "local_time": "2030-01-15T09:00:00"},
  "missed_policy": "skip",
  "concurrency_policy": "coalesce",
  "enabled": false
}
```

Le fuseau, la prochaine occurrence UTC, la politique de rattrapage et l'effet de l'action apparaissent dans la prévisualisation. Une création reste désactivée jusqu'à activation explicite. Un modèle pourra préremplir, sans auto-accepter une autorisation. Les permissions sont revalidées à l’activation puis à la prise en charge ; une cible clôturée est refusée sans duplication de mission.

Une occurrence porte trigger_id, occurrence_id, intended_at, received_at, source, request_digest, target_work_id, état, cause et éventuelle tentative liée. Une règle de provenance identifie ce qui relève du moteur, du fournisseur et de l'agent. Secret ou données métier sensibles n'appartiennent pas à ce journal.

## Parité des opérations à exposer

| Besoin | Web | CLI proposé |
| --- | --- | --- |
| Lire/exporter le brouillon | Ouvrir et exporter | plan draft show/export |
| Vérifier la modification | Prévisualiser | plan draft preview |
| Appliquer une révision | Appliquer | plan draft apply |
| Examiner un conflit | Comparer les changements | plan draft diff |
| Lister programmations | Programmes | trigger list/show |
| Créer une programmation | Préparer un programme | trigger preview/create |
| Activer ou suspendre | Activer ou Mettre en pause | trigger enable/pause |
| Suivre une occurrence | Dernières demandes | occurrence list/show |
| Annuler l'attente | Annuler cette demande | occurrence cancel |

Ces libellés CLI sont du vocabulaire de conception. L'inventaire A01 doit déterminer s'il faut des sous-commandes nouvelles ou étendre mission/planning existantes. Les snippets ne sont pas à copier dans un terminal avant livraison.

## Matrice des droits

Lecture : consulter états et preuves selon droits existants. Préparation : éditer un brouillon sans droit de lancement. Autorisation : appliquer le plan ou activer un déclencheur selon portée explicite. Exécution : conducteur prend une demande valide selon autorisation actuelle. Validation : contrôles et reviewer produisent preuves ; moteur accepte. Une skill ou un fournisseur sélectionné ne confère pas de droits supplémentaires.

## Reprise et arrêt

Reprise : identifier la cause, l'action corrective, la preuve de précondition et la prochaine opération permise. Les délais du fournisseur, les budgets cumulés et l'historique restent conservés. Un changement d'environnement vérifié peut justifier rejouer un contrôle, sans relancer le producteur. Un défaut du candidat demande correction et analyse d'impact.

Pause du déclencheur : aucune future demande ; occurrences en attente traitées par politique explicitement affichée. Annulation de l'occurrence : seulement si encore en attente et sans effet commencé. Arrêt de l'agent actif : utiliser le mécanisme public existant ; aucune annulation fictive ni récupération d'un droit consommé.
