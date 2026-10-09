# Méthodes produit KS et navigation des graphes

Autorisation : l’utilisateur a accepté le 9 octobre 2026 la migration des méthodes
KS et les trois niveaux application → parcours → story → tâches.

## Contexte vérifié

- `preparations.json` décrit les phases produit sans leurs méthodes détaillées.
- `agentWorkflow` fournit un pack figé par rôle, plafonné à 32 000 octets.
- `ActionPlan` et `ApprovedPlan` sont persistés avec la mission ; la préparation
  applique une révision transactionnelle et invalide les preuves impactées.
- `PilotGraph` replie des branches de dépendances, sans entités parcours/story.
- Les sources KS inspectées dans le dépôt CRM séparent framing, stories, revue,
  architecture existante/greenfield, design, recherche, plan, exécution et ship.

## Choix

Conserver une structure produit facultative dans le plan existant, plutôt qu’un
second magasin modifiable indépendamment. Les anciennes missions restent lisibles.
Les vues utilisent les mêmes tâches et preuves, sans agents ni acceptation propres.
Une story peut appartenir à plusieurs parcours ; une tâche peut servir plusieurs
stories. Les prérequis extérieurs restent consultables dans la vue détaillée.
Les limites de taille, de tâches, d’appels et de budget restent inchangées.

## Exécution et critères

1. Migrer la logique KS dans trois méthodes adaptées aux rôles, avec commandes de
   compatibilité et modèles documentaires versionnés/embarqués. Vérifier que les
   contextes reçus contiennent la méthode complète et conservent les permissions.
2. Ajouter au plan une structure produit explicite : parcours, stories de bout en
   bout, critères, complexité, dépendances et liens vers les tâches. Refuser les
   liens absents, doublons et cycles. Conserver cette structure après redémarrage.
3. Navigation : vue produit recherchable, parcours, story et graphe des tâches.
   Afficher les tâches partagées, prérequis externes, preuves périmées et décisions
   humaines sans déduire l’acceptation d’une fin de processus.
4. Vérifier sur fixtures isolées : compatibilité ancienne, révision/fraîcheur,
   graphe partagé, grand catalogue, FR/EN, clair/sombre et navigation clavier.
5. Documenter la correspondance KS/Swarm et ses adaptations ; vérifier le diff,
   les tests moteur/interface et l’installation neuve avant publication.

## Reprise

Une découverte qui change le contrat revient à ce plan. Une régression conserve
les données et l’historique ; aucune hausse de limite pour obtenir un résultat.
La recette utilise une mission isolée et un fournisseur simulé si un appel IA est
nécessaire. Elle ne prouve pas la qualité d’un fournisseur externe.
