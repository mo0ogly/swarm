# Parcours de création d’application

Swarm fournit maintenant les méthodes produit **Planification produit**,
**Réalisation produit** et **Revue produit**. Le modèle de nouvelle application,
d’évolution et d’interface sélectionne **Parcours de création d’application**. Les sources sont versionnées
et embarquées ; un projet vide n’a pas besoin de copier le dossier `.claude`.

## Étapes et méthodes

Le parcours distingue le cadrage du produit et le travail sur chaque fonctionnalité.
Les méthodes sont adaptées aux rôles de Swarm. Les modèles documentaires génériques sont dans
[`tools/agent-workflows/templates/ks`](../tools/agent-workflows/templates/ks).
Les instructions privées d’une application, sa configuration et ses clés ne sont
pas importées dans le pack.

### Cadrage : cinq étapes pour le produit

| Étape | Logique conservée | Méthode Swarm |
| --- | --- | --- |
| 1. Définir les besoins du produit | Mode, utilisateurs, problème, boucle de valeur, périmètre/exclusions, succès mesurable ; remplacement facultatif et vérifié | `product-planning` |
| 2. Décrire les parcours et fonctionnalités | Slices de valeur de bout en bout, critères, identifiants stables, dépendances, complexité et découpage d’un 5 | `product-planning` |
| 3. Vérifier la couverture des besoins | Couverture du PRD, exclusions, chevauchements, critères et ordre ; contexte distinct, constats conservés | `product-review` |
| 4. Définir l’architecture | Architecture existante vérifiée d’abord ; delta minimal ; greenfield explicite, choix de stack et ADR avant scaffold | `product-planning` + tâche de recherche `product-delivery` |
| 5. Définir les règles visuelles et composants | Direction existante/explicite, tokens, composants, états et règles ; blocage si la direction manque | `product-delivery` |

### Réalisation : six étapes pour chaque fonctionnalité

| Étape | Logique conservée | Méthode Swarm |
| --- | --- | --- |
| 6. Examiner l’existant | Code actuel, symboles, signatures, usages, persistance, impact, contrôles et inconnues | `product-delivery` |
| 7. Concevoir les écrans | Story et système obligatoires, agent ou brief externe, quatre états, accessibilité, gaps ; maquette comme référence | `product-delivery` |
| 8. Planifier les tâches | Tâches bornées, fichiers, dépendances, tests, gates, rollback et adoption explicite | `product-planning` + compatibilité existante |
| 9. Réaliser | Tâche autorisée, contrôle d’échec avant correction si pertinent, composants réels, retour au plan sur dérive | `product-delivery` |
| 10. Vérifier le résultat | Candidat et preuves de même révision, APIs vérifiées, design, régressions, sévérité et inconnues | `product-review` |
| 11. Livrer | Autorité explicite, checks/revue frais, PR sans doublon, protection de branche, merge/deploy confirmé | `product-delivery` |

Pour une évolution ciblée, cadrer son besoin avant ce cycle. Le suivi doit indiquer
l’étape réelle, les preuves manquantes et l’action suivante. **Le suivi persistant
des onze étapes et leurs transitions automatiques ne sont pas encore implémentés.**

### Adaptations nécessaires

Le planificateur Swarm n’a pas d’outils : la recherche dans le code et les livrables
documentaires sont des tâches d’exécutant. Le vérificateur reçoit les preuves dans
un contexte indépendant **sans outils** ; il ne prétend pas relancer les tests.
Les contrôles exécutables et l’intégration restent ceux du moteur. Une session
native autorisée peut, elle, exécuter ses propres contrôles de revue.

Les fichiers `validated: yes`, `Stories ready` et `Ship allowed` ne remplacent pas
l’adoption, les politiques de contrôle, les preuves fraîches ou l’autorisation de
départ du moteur. Les phases du pack sont une méthode, pas un nouvel ordonnanceur
qui exécute automatiquement onze commandes. Aucun budget n’est préautorisé.

Les méthodes intégrales adaptées sont incluses selon le rôle et leur empreinte
figure dans `workflow`. La préparation du parcours produit reçoit les trois sources pour
proposer le cycle, tout en restant limitée à l’analyse et au plan. Cela prouve
l’envoi du cadrage, pas l’obéissance d’un modèle ou la réussite d’un produit entier.

## Frontend, backend et parcours utilisateur

Une story porte une valeur utilisateur : « commander une pizza », par exemple.
Le frontend, l’API, la persistance et leurs tests sont des tâches de cette story,
plutôt que quatre stories techniques sans résultat utilisateur autonome.
Architecture, design system et contrats partagés peuvent être des tâches communes.
La conception couvre les champs/actions, empty/loading/error/success, le clavier,
les erreurs et l’affichage responsive. L’implémentation utilise les composants réels.

Pour une refonte, décrire l’existant avant le changement, le périmètre de parité,
les données à préserver, la migration, les contrôles et le rollback. Le pack ne
déclare pas automatiquement une reconstruction complète de l’application.

## Naviguer sans surcharger le graphe

1. **Application** : carte agrégée et liste des parcours, recherche et pagination
   de 20 éléments ; les tâches détaillées restent masquées.
2. **Parcours** : stories et leurs dépendances déclarées ; une seule carte ouverte.
3. **Story** : même graphe de tâches que le cockpit, filtré sur ses liens explicites.
   Objectif et critères sont consultables ; les prérequis extérieurs s’ouvrent dans
   l’inspecteur, les tâches dépendantes dans d’autres vues sont listées.

Une tâche partagée conserve le même identifiant et les mêmes preuves partout.
Les compteurs de tâches acceptées utilisent la validation actuelle du moteur ; ils
ne certifient pas à eux seuls qu’une story ou le produit complet est livré. Une
story sans tâches est **non planifiée**, jamais « terminée » par défaut.

Les tâches sans rattachement sont dans **Tâches communes et non classées**. Le rôle
de fondation n’est pas déduit d’un titre. **Toutes les tâches** et **Toutes les
dépendances** retrouvent la vue globale ; **Retrouver ma sélection** révèle également
une tâche extérieure. La navigation, la recherche et le repli ne changent pas la
mission et ne créent pas un sous-planificateur par story.

Les liens entre parcours agrègent les dépendances réelles des tâches et peuvent
aller dans les deux sens sans cycle dans le graphe des tâches. Les liens de stories
décrivent le parcours produit ; **les prérequis exécutables doivent aussi figurer
dans `depends` des tâches**. Aucun départ n’est autorisé par un simple lien visuel.

## Ajouter la structure au plan

Le JSON d’action version 1 accepte `product` facultatif et `phase` sur chaque tâche.
Les anciens plans restent compatibles. Les autres champs de tâche sont inchangés.

```json
{
  "mode": "existing",
  "journeys": [
    {"id": "purchase", "title": "Acheter", "goal": "Commander une pizza", "story_ids": ["s01-order"]}
  ],
  "stories": [
    {
      "id": "s01-order", "title": "Commander une pizza", "user": "Client",
      "value": "Recevoir une confirmation", "criteria": ["Commande persistée et confirmée"],
      "complexity": 3, "depends": [], "task_ids": ["T1", "T2"]
    }
  ]
}
```

Cet objet est la valeur du champ `product`, pas une requête de création autonome.
`task_ids` référence les IDs locaux du même plan. Une story peut être liée à
plusieurs parcours et une tâche à plusieurs stories. Le moteur refuse références
absentes/répétées, cycles de stories, IDs invalides, critères absents, complexité 5
ou 4 sans risque explicite. Une structure produit changée est conservée dans la
révision adoptée ; modifier le contrat d’une story invalide les tâches liées et
leurs dépendants selon le mécanisme de révision existant. Une simple modification
du libellé d’un parcours ne change pas les preuves.

Modes : `existing`, `greenfield`, `replacement`. Phases : `frame`, `requirements`,
`stories`, `story-review`, `architecture`, `design-system`, `research`, `design`,
`plan`, `implement`, `review`, `deliver`.

Les limites de 8 tâches par plan d’action et 16 000 octets par document restent
inchangées. Utiliser des incréments bornés ; les stories futures peuvent rester
sans tâches. Le moteur ne crée aucune story d’après les titres d’une ancienne mission.

## Recette reproductible

```sh
sh build.sh /tmp/swarm-product
python3 tests/product_recipe.py /tmp/swarm-product /tmp/swarm-product-project-neuf
/tmp/swarm-product --root /tmp/swarm-product-project-neuf web 127.0.0.1:18844
```

Choisir un port libre et une racine neuve. Ouvrir le lien de session affiché par le
serveur, puis Application → Acheter une pizza → Commander une pizza. La fixture
crée 25 parcours, 26 stories et 6 tâches verrouillées via le CLI public ; aucun
appel IA, départ d’agent, publication ou acceptation de livrable. Ce test ne prouve
pas une construction autonome complète d’application par un fournisseur réel.

Avec **Parcours de création d’application**, la structure `product` est obligatoire pour vérifier et adopter le plan. Son absence conserve le brouillon sans le déclarer prêt. Elle reste facultative avec les anciennes méthodes.

## Vues vérifiées

Captures de recette isolée : aucune IA ni exécution de tâche ; les données pizza servent uniquement à vérifier les vues.

![Journey and stories](screenshots/ks-product/journey-fr-light.jpg)

![Shared task context and prerequisites](screenshots/ks-product/story-en-light.jpg)

![Canonical task graph](screenshots/ks-product/tasks-en-light.jpg)

## Identifiants conservés pour compatibilité

Le libellé utilisateur est **Parcours de création d’application**. Les identifiants
`ks-product`, `ks-feature` et les fichiers de commandes `ks-*` restent des alias
techniques pour les appels existants. Ils ne désignent pas un autre moteur.
Les anciennes préparations restent lisibles ; une méthode dont le contenu change
doit être réappliquée et son plan revérifié avant lancement.
