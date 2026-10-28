# Deux interfaces pour le moteur Swarm

Décision produit et architecture adoptée avec l’utilisateur le 10 octobre 2026.
Swarm reste un produit autonome : Wattson est entièrement séparé et ne fait pas
partie de cette évolution.

## Décision

Développer ensemble deux UX, en les concevant dès le départ pour pouvoir les
déployer séparément :

- **Projets et développement** : objectifs, plans, tâches, code, tests, revues et
  livrables. Un projet vise un résultat dans un périmètre défini.
- **Processus métier** : définitions réutilisables, dossiers à traiter, règles,
  habilitations, validations et exceptions. Un processus traite des dossiers
  successifs selon une définition versionnée. La facturation est le premier cas
  envisagé pour préciser les besoins.

Les deux interfaces utilisent le même moteur Swarm et une API publique commune,
versionnée. La séparation doit exister dans le code et les contrats API ; deux
entrées dans un menu ne suffisent pas.

## Frontières à préserver

- Chaque application frontend possède sa navigation, ses écrans, son vocabulaire,
  son point d’entrée, son build et ses tests. Elle doit pouvoir être livrée sans
  embarquer l’autre interface.
- Le moteur commun porte l’orchestration, les transitions autorisées, les contrôles,
  les habilitations, les preuves, la reprise et le suivi des agents.
- Les deux UX utilisent les opérations publiques. Aucune ne lit ou ne modifie
  directement le stockage du moteur ; les contrôles restent applicables sans UX.
- Partager les composants utiles : identité visuelle, jetons sémantiques, graphe,
  formulaires et accessibilité. Garder les parcours propres à chaque usage dans
  leur application, sans importer les écrans métier d’une UX dans l’autre.
- Les règles et autorisations sont définies par les responsables humains. Les
  agents proposent ou exécutent des actions dans les limites autorisées ; leurs
  explications ne remplacent pas les contrôles du moteur.

## Organisation initiale

Commencer dans le même dépôt, avec un accueil commun : « Que voulez-vous
orchestrer ? », puis les entrées **Projet** et **Processus métier**. Les modèles
sont proposés selon l’usage choisi. Un seul serveur peut héberger les deux UX,
par exemple sous `/projets` et `/processus` ; ces chemins sont une proposition,
pas des routes déjà livrées.

À terme, servir les UX sur des adresses et avec des cycles de livraison distincts,
sans réécrire leurs parcours ni dupliquer le moteur. La séparation frontend
n’impose pas de créer deux moteurs ou deux stockages ; les besoins d’isolation
des données et d’identité devront être spécifiés avant une exploitation métier.

## Processus fonctionnels

L’espace Processus distingue trois objets :

1. **Définition du processus** : graphe des étapes, règles, version, agents
   autorisés, points de validation humaine et conditions de reprise.
2. **Dossiers et exécutions** : chaque facture ou lot suit une version identifiée
   du processus, avec son état et les actions attendues.
3. **Supervision** : actions réellement réalisées, décisions d’autorisation ou de
   refus, incidents, preuves et consommation connue ou inconnue.

Le concepteur travaille sur le graphe. L’opérateur dispose d’une vue orientée
dossiers : à traiter, en contrôle, en attente de validation, bloqué, terminé.
Il ouvre le graphe pour comprendre le parcours d’un dossier. Le vocabulaire
développement et les détails techniques des agents ne doivent pas devenir des
prérequis pour utiliser l’UX métier.

Pour la facturation, le parcours envisagé est : recevoir, vérifier, rapprocher,
approuver, transmettre le paiement, confirmer. Les règles exactes, les connecteurs,
les habilitations et les effets externes restent à spécifier ; ce parcours ne
constitue pas une autorisation de paiement réel.

## État et prochaine étape

Cette décision fixe la cible ; elle ne décrit pas deux applications déjà livrées.
Le banc de facturation vérifie certains mécanismes du moteur et ne constitue pas
une UX métier complète. Les campagnes scriptées ne démontrent pas l’autonomie
d’agents LLM réels.

Avant l’implémentation, définir les parcours et responsabilités de la facturation,
les contrats API nécessaires et les frontières des deux frontends. Vérifier la
séparabilité par des builds, tests et déploiements indépendants, ainsi que
l’application des mêmes contrôles via API sans passer par les interfaces.

## Décision du 10 octobre 2026 — modèles de l’UX développement

L’utilisateur adopte les modèles comme **surcouche du parcours commun en onze
étapes**, décrit dans [PRODUCT-WORKFLOW.md](PRODUCT-WORKFLOW.md). Un modèle
spécialise le contexte, les questions de cadrage, les livrables, les risques et
les contrôles ; il ne remplace pas la méthode ni les autorisations du moteur.
Les cinq étapes de cadrage sont communes au produit ; les six suivantes sont
un cycle répété **pour chaque fonctionnalité** (cinq étapes communes, puis six
par fonctionnalité, et non onze étapes exécutées une seule fois pour tout le
produit). Les étapes non applicables sont justifiées, les acquis
existants sont examinés avant d’être réutilisés. Reprendre un projet ne signifie
ni reconstruire son code ni déclarer ses preuves automatiquement valides.

Dans l’accueil développement, l’intention (construire, faire évoluer, corriger)
précède le choix éventuel d’un modèle orienté sujet. Partir de zéro et reprendre
un projet sont des points de départ, pas deux méthodes concurrentes. Chaque
modèle doit être fourni en français et en anglais, avec un exemple, son périmètre,
les réponses attendues, les critères observables et les preuves nécessaires.

La première sélection comporte six sujets : outil interne, portail client, API
et intégration, gestion des stocks, traitement de données et tableau de bord.
Chaque sujet propose créer, faire évoluer, corriger ou migrer. Aucun quota
arbitraire de modèles n’est adopté. Les modèles des processus fonctionnels
appartiendront à leur UX distincte ; Wattson reste hors de ce périmètre.

**État :** décision d’architecture adoptée ; l’accueil et le catalogue de
préparation existant sont raccordés. Le catalogue spécialisé est implémenté avec une projection commune CLI/API.
Le suivi persistant des étapes reste à réaliser. Aucun statut d’étape ne doit être
inventé à partir d’un simple libellé de modèle.

## Précision — modèles de cadrage et modèles de travail

L’utilisateur précise qu’un modèle de travail doit porter une organisation
réutilisable : groupes d’actions, dépendances, orientations des décisions,
validations et reprises. L’adaptation au besoin puis l’adoption humaine restent
nécessaires avant exécution. Cinq sujets restent des **modèles de cadrage**
(questions, livrables, preuves, méthode). **Portail client v2** contient désormais
un modèle de travail : cinq incréments FR/EN, graphes inspectables, plans
ActionPlan v1 adaptés aux réponses et insérés dans la préparation. Ses dépendances
locales sont contrôlées par le moteur ; adoption, validation et départ restent
les opérations existantes. Les prérequis entre incréments nécessitent une décision
explicite et des preuves ; aucun ordonnanceur conditionnel nouveau n’est livré.
Le moteur contrôle les tâches et les dépendances d’un plan adopté ; cette capacité
existante ne prouve pas que chaque sujet contient déjà ces structures ni que
toute forme de branchement conditionnel est prise en charge. La recette publique de Portail client couvre la projection CLI/API/UX, la
conservation des réponses, l’enregistrement et la conversion en missions retenues
au départ. L’extension des autres sujets reste un lot distinct.
