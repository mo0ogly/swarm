# Recette du dépôt autonome — 19 septembre 2026

## Périmètre

Recette depuis un clone du dépôt public `mo0ogly/swarm`, révision
`fd6399823f338b78eaef8c3effaab072a626278d`, sur Linux avec Docker Engine
24.0.7 et Compose 2.21. Les recettes de processus ont ensuite été adaptées
au contrat actuel de planification hiérarchique. Aucun changement du moteur.

## Résultats observés

| Contrôle | Résultat |
|---|---|
| Compilation et suite Go | Réussi |
| Tests JavaScript du graphe, des rôles et du rafraîchissement | Réussi |
| Recette CLI : processus séparés, doublons, export/import | Réussi |
| Construction des ressources frontend | Réussi |
| Interface IA et connexions, deux thèmes, API simulée | Réussi |
| Installation native dans un dossier isolé | Réussi |
| Construction et démarrage Docker, session navigateur et CLI | Réussi |
| Refus de réinstallation sur service actif | Réussi |
| Persistance mission et dossier des agents après recréation | Réussi |
| Propriété des fichiers avec UID/GID utilisateur | Réussi |
| Deux tâches successives validées sans intervention | Réussi |
| Trois exécutants, deux productions parallèles puis synthèse | Réussi |
| Même scénario avec espace partagé : productions sérialisées | Réussi |
| Arrêt brutal et relance du serveur pendant les productions | Réussi |

Les trois scénarios d’équipe vérifient trois remises de rapports, trois revues
indépendantes, la distinction producteur/contrôleur, les dépendances et la
clôture du responsable. Aucune intervention après lancement. Les assertions
portent sur les états réels enregistrés et sur les intervalles des processus.

## Corrections des recettes

L’ancienne recette de validation créait des tâches sans responsable ; le moteur
refusait correctement leur lancement. Elle utilise maintenant les commandes
publiques de planification, les critères hérités et les contrôles autorisés.
La nouvelle recette `organized_coordination_process.py` vérifie les remises
hiérarchiques. L’ancienne `autonomy_coordination_process.py`, conservée,
repose sur un protocole d’échanges entre pairs qui ne correspond pas au parcours
hiérarchique actuel ; elle ne constitue pas une preuve de ce parcours.

Un premier rapport de fixture, réduit à deux caractères, a aussi été rejeté par
la revue indépendante. La fixture fournit désormais un texte suffisamment
explicite, comparé intégralement par un contrôle ; les protections du moteur
n’ont pas été relâchées.

## Reproduire

```sh
make build test smoke
make frontend
npm run test:connections
make test-install
make test-process
```

`test-install` requiert Docker accessible et aucun `deploy/install.env`
préexistant. Les recettes utilisent des projets isolés et des agents factices.
Les journaux de serveur peuvent contenir des liens de session : ne pas les publier.

## Limites

Les processus agents et le relecteur sont déterministes : aucun appel à Claude,
Codex, Skynet ou un modèle payant. Le nom d’exécutable `claude` dans la fixture
sert seulement à exercer le protocole de l’adaptateur. Ces résultats ne prouvent
ni la qualité d’un modèle, ni son authentification, ni ses permissions réelles.
L’image Docker n’embarque aucun programme agent ni poids de modèle. Le parcours
avec installation d’un fournisseur réel reste à recetter. Docker Desktop,
Windows et les moteurs Docker distants ne sont pas couverts.
