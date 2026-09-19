# Utiliser Swarm, du besoin au résultat

Ce guide décrit le parcours web et ses équivalents en ligne de commande.
Vous pouvez préparer et piloter une mission depuis Swarm sans passer par une
conversation externe avec Codex. Les programmes agents doivent cependant être
installés et authentifiés dans l’environnement où Swarm les exécute.

Les captures illustrent des données de démonstration, sans agents réels en cours.

## Choisir la langue

Le sélecteur **Langue** propose Français et English dans le cockpit, la préparation
et la session d’agent. Le choix est mémorisé dans le navigateur. `?lang=en` ou
`?lang=fr` permet un choix explicite dans le lien. Le changement recharge la vue :
enregistrez les brouillons avant de basculer. Une préparation avec saisie ou
opération en attente refuse cette navigation pour préserver votre travail.

Dans le CLI : `swarm --lang en …`, ou `SWARM_LANG=en swarm …`.
L’option explicite prime sur la variable ; le français reste la valeur par défaut.
Les commandes, codes et clés JSON restent identiques. Les titres, rapports et
sorties des agents conservent leur langue d’origine.

[Read the English user guide](docs/en/USER-GUIDE.md).

En cas de refus du fournisseur : [comprendre et traiter une attente de quota IA](docs/PROVIDER-QUOTAS.md).

## Sommaire

1. [Ouvrir Swarm](#1-ouvrir-swarm)
2. [Connecter les IA](#2-connecter-les-ia)
3. [Préparer une mission](#3-préparer-une-mission)
4. [Vérifier l’équipe et lancer](#4-vérifier-léquipe-et-lancer)
5. [Suivre le travail](#5-suivre-le-travail)
6. [Comprendre les états et valider](#6-comprendre-les-états-et-valider)
7. [Résoudre un blocage](#7-résoudre-un-blocage)
8. [Modifier une mission](#8-modifier-une-mission)
9. [Pause, arrêt et nettoyage](#9-pause-arrêt-et-nettoyage)
10. [Utiliser le terminal](#10-utiliser-le-terminal)
11. [Limites et données](#11-limites-et-données)

## 1. Ouvrir Swarm

Suivez d’abord le [guide d’installation](INSTALL.md), Docker ou natif.
Choisissez le **dossier du projet à faire examiner ou modifier**, qui peut être
différent du dépôt du logiciel Swarm. Ce dossier contient son propre état `.swarm/`.

En natif, depuis le dépôt Swarm compilé :

```sh
./bin/swarm --root /chemin/du/projet init
./bin/swarm --root /chemin/du/projet providers init
./bin/swarm --root /chemin/du/projet web 127.0.0.1:18787
```

Ouvrez le **lien de session affiché dans le terminal**. En Docker, retrouvez-le
avec `docker compose --env-file deploy/install.env logs --tail 20 swarm`.
Ce lien donne accès au cockpit : ne le mettez pas dans vos captures publiques.
Le sélecteur permet de consulter plusieurs missions dans le même onglet.

Si le navigateur est sur un autre ordinateur, `127.0.0.1` désigne cet ordinateur,
pas le serveur : utilisez le [tunnel décrit dans l’installation](INSTALL.md#réseau-et-accès-au-navigateur).

## 2. Connecter les IA

Ouvrez **IA et connexions**. Deux sortes de connexions ont des usages différents :

| Connexion | Utilité | Ce qu’il faut fournir |
|---|---|---|
| API compatible `/chat/completions` | Préparer, planifier, examiner un rapport | Adresse, identifiant exact du modèle, clé si nécessaire |
| Programme agent avec outils | Lire et modifier les fichiers, exécuter des commandes | Programme installé, authentification et configuration du fournisseur |

Swarm ne contient ni Claude Code, ni Codex, ni Skynet, ni les poids des modèles.
Dans Docker, un programme installé sur votre ordinateur n’est pas automatiquement
installé dans le conteneur. Voir [Configurer les IA dans Docker](INSTALL.md#2-configurer-les-ia-dans-docker).

### Ajouter une API

1. Choisissez **Ajouter une IA**.
2. Renseignez un identifiant local, un nom lisible, l’adresse de base du service
   et l’identifiant exact du modèle disponible chez ce fournisseur.
3. Utilisez **Tester la connexion**. Ce test envoie une courte question ; il peut
   être facturé. Il n’envoie aucun document de mission.
4. Enregistrez : le test seul ne sauvegarde pas la connexion.
5. Retrouvez-la sous le nom `api-<identifiant>` dans les choix de préparation
   et de planification.

Une réponse au test prouve la connexion texte, pas la capacité à modifier des
fichiers ni à respecter tous les formats structurés demandés au responsable.
Une API texte seule ne peut pas être l’exécutant d’une mission de code.

![Configuration d’une connexion IA](docs/screenshots/connexion.png)

### Choisir les modèles

Les niveaux utilisent la politique enregistrée du fournisseur. Vérifiez le modèle
résolu avant le départ ; Swarm ne choisit pas librement un modèle plus puissant.
Une connexion API personnalisée utilise son modèle enregistré pour les trois
niveaux. Elle ne représente donc pas trois modèles différents.

Lors de la création des missions, choisissez séparément **IA du responsable et
du vérificateur** et **Agent avec outils pour les exécutants**. Le même programme
peut servir aux deux si ses capacités le permettent.

## 3. Préparer une mission

Depuis le cockpit, choisissez **Préparer un projet**.
Décrivez le résultat attendu, le dossier concerné, les limites et la façon de
vérifier la réussite. Exemple à adapter :

> Ajouter une recherche dans la liste des documents. Conserver les filtres
> existants et les deux thèmes. Vérifier les recherches sans résultat et les
> caractères accentués. Livrer le code, les tests et un rapport des résultats.

1. Choisissez le fournisseur et le niveau de préparation.
2. Échangez avec l’IA, puis **relisez et adoptez le brief**.
3. Demandez un plan et répondez aux décisions encore ouvertes.
4. Avec **Modifier les missions**, précisez chaque titre, périmètre, livrable,
   critère de réussite et dépendance.
5. Utilisez **Vérifier le plan** et corrigez les problèmes signalés.
6. Choisissez **Relire l’équipe proposée**, puis vérifiez le responsable, les
   exécutants, la revue, les IA, l’espace de travail, les limites et le mode de
   validation. Exécutez le prévol, puis donnez l’autorisation unique récapitulative.

La vérification du plan contrôle sa cohérence ; elle ne prouve pas que le travail
est réalisé. Aucun travail ni agent n’est créé avant l’autorisation. Après celle-ci,
seules les missions éligibles peuvent partir ; dépendances, budget, pause et espace
occupé restent contrôlés.

![Préparation d’une mission](docs/screenshots/preparation.png)

### Choisir qui valide

- **Moi — revue humaine** : vous examinez les résultats et les acceptez.
  L’équipe peut avancer automatiquement jusqu’à cette étape.
- **Le moteur — contrôles autorisés** : vous autorisez à l’avance les commandes
  exactes qui démontrent les critères. Une couverture manquante ou un contrôle
  échoué empêche l’acceptation automatique.

Le vérificateur IA des nouvelles préparations examine le rapport dans une
session distincte. Son avis ne remplace ni les contrôles exigés ni l’acceptation
humaine choisie. Pour un premier essai dont les critères sont qualitatifs,
la revue humaine rend explicite ce que vous devez encore examiner.

## 4. Vérifier l’équipe et lancer

Avant d’autoriser le démarrage, vérifiez ces responsabilités :

| Rôle | Responsabilité | Comment le lire |
|---|---|---|
| Responsable / orchestrateur | Organiser, recevoir les retours et décider de la suite | Carte de responsabilité, décisions et état de son périmètre |
| Responsable de branche | Organiser une partie déléguée, si elle existe | Responsabilité rattachée à son périmètre |
| Exécutant | Réaliser une tâche et produire son livrable | Carte de tâche, tentative et session |
| Vérificateur indépendant | Examiner les critères et le rapport | Carte de vérification et avis liés à la tentative |
| Contrôles du moteur | Exécuter les vérifications autorisées | Résultats de contrôles et décision de validation |

Une carte de responsable visible ne signifie pas qu’un appel IA tourne en
permanence. Une tâche intitulée « revue » n’est pas, à elle seule, un vérificateur
configuré. Les anciennes missions peuvent avoir une organisation incomplète :
lisez les rôles manquants au lieu de supposer qu’ils sont créés automatiquement.

Après **Relire et autoriser le démarrage**, ouvrez le pilotage et utilisez
**Lancer la mission**. Relisez le fournisseur, le dossier, les créneaux et les
limites avant confirmation. L’autorisation du plan et le lancement sont deux étapes.

Le nombre de créneaux fixe un maximum de tentatives simultanées. Des dépendances
non validées ou un dossier déjà occupé peuvent réduire le parallélisme réel.
**Lancer tout** démarre les tâches prêtes à cet instant ; pour un enchaînement
durable, utilisez la mission et vérifiez que son conducteur est observé.

## 5. Suivre le travail

Commencez par le résumé du **Pilotage des agents** : ce qui se passe, la prochaine
action et qui doit agir. Consultez ensuite le graphe ou la liste.

- Les flèches de dépendance vont du prérequis vers la tâche qui en dépend.
  Elles tiennent compte du verdict du moteur et de la fraîcheur des preuves.
- Les liens de responsabilité relient les rôles à leurs périmètres. Ne déduisez
  pas le sens d’un lien de sa seule couleur ou de son seul style : lisez sa légende.
- **Horizontale / Verticale** change l’orientation, **Simplifiées / Détaillées**
  change les informations visibles. Aucun de ces réglages ne réordonne le travail.
- **+ / −** replie ou déplie une branche. Cela ne supprime aucune tâche.
- Les filtres et **Vue d’ensemble** aident à retrouver une branche hors écran.
- Un clic ouvre les détails. **Voir l’agent travailler** ouvre sa session ;
  un double-clic sur une tâche ouvre sa session lorsqu’elle existe.

![Rôles, tâches et flèches](docs/screenshots/agents-horizontal.png)

Dans la session, lisez l’heure, le type d’événement et le message. Les sorties
reçues décrivent l’activité observable : elles ne donnent pas accès au raisonnement
interne de l’IA. La capture détaillée est facultative ; sans elle, tout le texte
brut du fournisseur n’est pas nécessairement conservé.

![Détail d’un agent](docs/screenshots/agent-detail.png)

**Ces captures utilisent des données de démonstration**, sans agents réels en
cours. Les fournisseurs, missions et états de votre installation seront différents.

## 6. Comprendre les états et valider

| Indication | Signification | Suite |
|---|---|---|
| À faire | La tâche n’a pas encore commencé | Lire ses conditions de départ |
| En cours | Une tentative est en cours dans le suivi | Vérifier aussi le processus et son dernier signal |
| À vérifier | Un résultat a été soumis | Lire rapport, avis et contrôles |
| Bloquée | Un obstacle empêche la suite | Ouvrir le motif et la reprise proposée |
| Terminée et validée | Le résultat a été accepté | Les dépendances peuvent être satisfaites si les preuves restent actuelles |
| Acceptée — à revalider | Les preuves d’une acceptation ont changé | Refaire les vérifications concernées |
| Dérogation / Abandonnée | Décision explicite, différente d’une réussite | Lire sa justification |

**Un processus “Terminé” n’est pas une tâche “Terminée et validée”.** De même,
un résultat récent n’est pas une preuve que le processus travaille encore.
Swarm distingue exécution, activité reçue et validation.

En revue humaine, ouvrez le résultat à examiner, lisez le rapport et les critères,
puis utilisez les actions de revue et d’acceptation proposées dans la fenêtre.
Une “gate” désigne un contrôle de passage : si elle refuse la livraison, consultez
les preuves manquantes ou périmées. Acquitter une notification signifie seulement
que vous l’avez lue.

En validation automatique, le moteur attend les conditions enregistrées ;
ne relancez pas un agent uniquement parce qu’une revue est encore en attente.
La mission hiérarchique n’est achevée que lorsque les résultats sont traités
et que les responsables ont clos leurs périmètres.

## 7. Résoudre un blocage

Ouvrez la tâche signalée et son diagnostic. **Expliquer avec l’IA** peut aider à
comprendre le contexte et proposer une action. Examinez l’effet de cette action
avant de l’appliquer ; un conseil n’est pas une validation.

| Message ou symptôme | Vérification et action utile |
|---|---|
| Fournisseur absent | Installer et authentifier l’agent dans le bon environnement ; contrôler sa déclaration |
| Seulement des exécutants | Vérifier l’organisation et les rôles manquants ; ne pas contourner le refus de lancement |
| Espace occupé | Identifier la tentative qui réserve le dossier ; attendre sa fin ou examiner son arrêt |
| Dépendance non validée ou périmée | Ouvrir le prérequis et traiter sa revue ou ses preuves |
| Tentative terminée sans rapport | Examiner la sortie, le livrable attendu et son emplacement avant reprise |
| Erreurs d’outils répétées | Corriger droits, ressource ou test en échec avant de relancer ; augmenter le plafond ne corrige pas la cause |
| Signal perdu / processus non confirmé | Examiner la session et le poste d’exécution ; éviter de lancer un doublon |
| Conducteur absent | Vérifier que le serveur web du même projet fonctionne ; l’autorisation seule ne fait pas avancer la mission |
| Contexte trop volumineux | Réduire les documents transmis ou repartir d’un brief plus court ; aucun envoi tronqué n’est promis |
| Coût non rapporté | Le fournisseur n’a pas transmis cette mesure ; cela ne signifie pas gratuit |
| Refus après confirmation | Relire l’état : la mission a pu changer entre l’aperçu et l’action |

Corrigez la cause, puis utilisez la reprise proposée. Une répétition identique
sans changement des conditions risque de produire le même échec.

## 8. Modifier une mission

Utilisez **Réviser ce Swarm** pour revenir à sa préparation source.
Modifiez le brouillon, vérifiez le nouveau plan, puis choisissez
**Comparer et appliquer la révision**. Relisez tâches ajoutées, modifications
et résultats à revérifier.

Le brouillon seul ne change pas la mission. Une révision appliquée conserve
l’historique, peut rouvrir des résultats, met la mission en pause et verrouille
les prochains départs. Relisez et autorisez le plan avant de reprendre.
Les tentatives actives ou certaines délégations peuvent empêcher l’application :
le refus doit être résolu avant une nouvelle comparaison.

Pour modifier les flèches, modifiez les **dépendances du plan** et vérifiez-le.
Le déplacement visuel ou l’orientation du graphe ne change pas l’ordonnancement.

## 9. Pause, arrêt et nettoyage

- **Mettre en pause** suspend les prochains départs ; les agents déjà lancés
  continuent. Pour arrêter une tentative, utilisez son action d’arrêt et vérifiez
  ensuite la confirmation de fin du processus.
- Fermer l’onglet ne coupe pas les agents. Arrêter le serveur ne garantit pas
  l’arrêt des processus agents ; arrêter le conteneur interrompt son environnement.
- **Reprendre la mission** réautorise la progression, sous réserve des conditions.

Dans **Gérer les missions**, utilisez recherche et filtres :

| Action | Effet |
|---|---|
| Archiver | Range la mission et produit une archive ZIP ; conserve ses données |
| Restaurer | Réactive une archive ou récupère une mission en corbeille |
| Purger l’historique | Retire les anciens journaux et sorties selon la rétention choisie, sans retirer preuves et reçus |
| Supprimer | Place les données internes en corbeille récupérable ; ne supprime pas les fichiers du projet |

L’aperçu ne modifie rien. Lisez volumes et exclusions avant confirmation.
La suppression demande le nom exact. Une activité en cours peut empêcher
l’opération : arrêtez ou résolvez l’activité concernée, puis demandez un nouvel aperçu.
Consultez l’installation pour les [sauvegardes et mises à jour](INSTALL.md#4-données-arrêt-et-mise-à-jour).

## 10. Utiliser le terminal

Le web et le CLI partagent l’état **seulement s’ils ciblent le même dossier projet**.
Dans les exemples, remplacez les chemins et identifiants ; ne saisissez pas les
mots `IDENTIFIANT` littéralement.

Depuis le dépôt Swarm compilé :

```sh
./bin/swarm --root /chemin/du/projet work list
./bin/swarm --root /chemin/du/projet work show IDENTIFIANT
./bin/swarm --root /chemin/du/projet mission status IDENTIFIANT
./bin/swarm --root /chemin/du/projet planning show IDENTIFIANT
./bin/swarm --root /chemin/du/projet agent list IDENTIFIANT
./bin/swarm --root /chemin/du/projet agent logs IDENTIFIANT_AGENT
./bin/swarm --root /chemin/du/projet console IDENTIFIANT
```

Pause et reprise, uniquement sur la mission que vous avez sélectionnée :

```sh
./bin/swarm --root /chemin/du/projet mission pause IDENTIFIANT
./bin/swarm --root /chemin/du/projet mission resume IDENTIFIANT
```

Dans Docker, remplacez le préfixe par :

```sh
docker compose --env-file deploy/install.env exec swarm swarm --root /workspace work list
```

Pour les scripts, ajoutez `--json` avant la commande. L’aide thématique est
accessible avec `swarm aide pilotage` ou `swarm aide validations` si le binaire
est installé dans votre PATH.

Les créations et révisions CLI utilisent des documents JSON versionnés : voir
[les contrats de préparation](PREPARATION-UX.md#cli--mêmes-mutations-mêmes-garanties)
et la [référence](REFERENCE.md). Les mutations exigent la révision courante et
un identifiant d’opération ; ne remplacez pas aveuglément une révision refusée.
Le terminal n’a pas tous les formulaires web. Le test interactif de connexion API,
notamment, se fait dans le web.

## 11. Limites et données

Les missions, connexions et journaux sont locaux dans `.swarm/`. Les clés API
sont dans un fichier réservé au compte du processus, **sans chiffrement applicatif**.
Ne publiez ni ce dossier ni les liens de session. Les sorties capturées peuvent
contenir du contenu sensible du projet.

Les méthodes APEX, KS et PDCA nécessitent leurs ressources dans le projet piloté ;
elles ne sont pas toutes incluses. La préparation standard ne crée pas
implicitement des copies Git isolées : ne confondez pas ce parcours avec le mode
avancé de dépôt Git géré, décrit dans la référence.

Les tests avec des agents simulés ne remplacent pas une recette de votre
fournisseur réel et de votre projet.

Pour approfondir : [préparation](PREPARATION-UX.md), [cockpit technique](COCKPIT.md),
[référence CLI](REFERENCE.md), [installation](INSTALL.md).
