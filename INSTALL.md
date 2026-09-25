# Installer Swarm

[English installation guide](docs/en/INSTALL.md).

Swarm propose deux installations **sous Linux** : Docker pour un environnement dédié, ou un binaire natif pour utiliser les outils déjà installés sur votre machine. Le script ne demande pas `sudo`, ne modifie pas votre profil shell et n’installe pas Docker à votre place.

Après installation, suivre le [guide utilisateur](GUIDE-UTILISATEUR.md) pour connecter une IA, préparer une mission et lire ses résultats.

## 1. Installation Docker

Prérequis : Git, Bash, Docker Engine démarré et accessible à votre utilisateur, Docker Compose v2 avec `up --wait`. Les téléchargements des images et dépendances nécessitent Internet. Vérifiez :

```sh
docker version
docker compose version
```

Si nécessaire, suivez les instructions officielles pour [Docker Engine](https://docs.docker.com/engine/install/) et le [plugin Compose](https://docs.docker.com/compose/install/linux/).

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm

# Remplacez ce chemin par votre projet existant, ou créez un projet d’essai.
mkdir -p "$HOME/projets/mon-projet"
./install.sh --project "$HOME/projets/mon-projet"
```

Le script construit l’image, initialise le projet si nécessaire et démarre le cockpit. Il affiche ensuite **le lien de session à ouvrir dans votre navigateur**. Ce lien donne accès à votre cockpit : ne le publiez pas.

Pour retrouver ce lien :

```sh
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

### Ce qui est installé

- Un binaire Swarm compilé depuis votre copie du dépôt, avec les ressources web embarquées.
- Un environnement contenant Node.js 22, Python 3, Git, curl et les certificats TLS.
- Un conteneur exécuté avec votre UID/GID, pour conserver la propriété des fichiers.
- Le projet monté en écriture dans **`/workspace`**.
- Un dossier personnel persistant dans **`/home/swarm`** pour les outils et authentifications des agents.

**L’image de base ne contient aucun fournisseur d’agent préinstallé ni aucune clé IA.** Elle permet de préparer le cockpit et d’ajouter une connexion API. Un agent disposant d’outils reste nécessaire pour réaliser des modifications de code.

### Options

```sh
# Choisir un autre port et un stockage séparé pour les agents.
./install.sh --project "$HOME/projets/mon-projet" \
  --port 18788 --agent-home "$HOME/.local/share/swarm/agents-demo"

# Construire et préparer la configuration sans démarrer.
./install.sh --project "$HOME/projets/mon-projet" --no-start

docker compose --env-file deploy/install.env up -d --wait
```

Le fichier local `deploy/install.env` contient les chemins, le port et les UID/GID. Il est exclu de Git et créé avec des permissions privées. Le script refuse de réinstaller lorsque le service Compose est déjà actif : arrêtez d’abord proprement les missions.

### Réseau et accès au navigateur

Swarm conserve une écoute **sur `127.0.0.1` uniquement**. Compose utilise donc `network_mode: host`, sans publication `ports:`. Cette configuration partage le réseau de l’hôte tout en conservant la séparation des fichiers et processus du conteneur. Voir [la documentation Docker du réseau hôte](https://docs.docker.com/engine/network/drivers/host/).

Le parcours décrit et testé ici vise **Docker Engine sur Linux avec un moteur local**. Docker Desktop, les moteurs Docker distants et Windows ne font pas partie de cette recette. Ne remplacez pas l’écoute par `0.0.0.0` : le serveur la refuse.

Sur une machine distante, ouvrez un tunnel depuis votre poste :

```sh
ssh -L 18787:127.0.0.1:18787 utilisateur@serveur
```

Puis ouvrez le lien de session avec `127.0.0.1:18787` dans votre navigateur local. Si ce port local est déjà occupé, adaptez les deux côtés du tunnel et le port Swarm pour conserver un accès cohérent avec l’adresse du serveur.

## 2. Configurer les IA dans Docker

### Connexions API

Dans **IA et connexions**, ajoutez le nom, l’adresse du service, l’identifiant du modèle et, si nécessaire, une clé. Testez la connexion avant de l’utiliser.

Le protocole attendu est compatible avec `POST /chat/completions`. Une connexion API peut servir à préparer, planifier ou examiner un rapport. Elle ne donne pas automatiquement des outils d’accès aux fichiers.

Avec le réseau hôte Linux, une API locale peut être joignable par exemple à `http://127.0.0.1:11434/v1`, si vous avez effectivement lancé un service compatible à cette adresse.

### Skynet depuis le conteneur, par le proxy de l’hôte

Le harnais Skynet installé **sur l’hôte** fournit un proxy LiteLLM compatible OpenAI sur `127.0.0.1:4010`. Grâce à `network_mode: host`, le conteneur joint ce proxy à la même adresse : aucune modification de l’image n’est nécessaire, et les clés Skynet restent sur l’hôte.

Dans **IA et connexions**, ajoutez une connexion :

| Champ | Valeur |
| --- | --- |
| Adresse du service | `http://127.0.0.1:4010/v1` |
| Modèle | Un nom de route exposé par le proxy |
| Clé | La clé maîtresse locale du proxy (`general_settings.master_key` dans sa configuration LiteLLM) |

Lister les routes disponibles depuis le conteneur :

```sh
docker compose --env-file deploy/install.env exec swarm \
  sh -c 'curl -s -H "Authorization: Bearer $CLE_PROXY" http://127.0.0.1:4010/v1/models'
```

Remplacez `$CLE_PROXY` par la clé maîtresse du proxy, sans l’enregistrer dans un fichier suivi par Git.

Cette connexion prépare, planifie et examine ; elle ne modifie pas les fichiers. Pour un agent Skynet disposant d’outils dans le conteneur, `skynet_harness` doit y être installé et configuré : ce parcours n’est pas couvert par cette recette.

Les routes du proxy suivent le catalogue publié par Skynet. Une erreur `404` du service amont signale en général des routes périmées : mettez à jour le harnais **sur l’hôte**, relancez son installateur, puis vérifiez avec `skynet-doctor`. Swarm ne bascule jamais automatiquement vers un autre modèle.

### Agents capables de modifier le projet

Le dépôt GitHub contient les adaptateurs de Swarm, pas les programmes Claude Code, Codex ou Skynet, ni les poids des modèles. La commande `skynet_harness` est reconnue par le routage des modèles lorsqu’elle est installée et déclarée comme fournisseur ; cette reconnaissance ne constitue pas une installation ni une recette de bout en bout.

Les commandes installées sur l’hôte ne deviennent pas automatiquement disponibles dans le conteneur. Installez les agents **dans le conteneur**, selon la documentation de leur éditeur, puis authentifiez-les dans cet environnement.

Ouvrir un terminal dans le service :

```sh
docker compose --env-file deploy/install.env exec swarm bash
```

Pour un agent distribué par npm, le modèle de commande est `npm install -g NOM_DU_PAQUET@VERSION` : remplacez ces paramètres par le paquet officiel et la version que vous avez choisis. Le préfixe npm `/home/swarm/.local` est persistant et son dossier `bin` figure dans le PATH.

Les composants système supplémentaires doivent être ajoutés dans une image dérivée. Ils ne persistent pas après recréation s’ils sont installés seulement dans la couche d’un conteneur.

Swarm découvre les CLI connues présentes au **premier** `providers init`. Si vous ajoutez un fournisseur après le premier lancement, conservez le fichier existant et ajoutez ou adaptez sa définition dans `/workspace/.swarm/providers.json`. `providers init` ne l’écrase pas. Consultez le [contrat des fournisseurs](REFERENCE.md) pour les arguments et la transmission des variables d’environnement.

Vérifier la configuration depuis le dépôt, sur l’hôte :

```sh
docker compose --env-file deploy/install.env exec swarm \
  swarm --root /workspace providers show
```

Dans les profils de tâches, utilisez les chemins **visibles dans le conteneur**, notamment `/workspace`. Les chemins absolus d’une ancienne installation native ne sont pas automatiquement réécrits.

Le conteneur ne monte ni le socket Docker, ni les clés SSH, ni le dossier personnel complet de l’hôte. Les accès Git privés et les outils supplémentaires se configurent selon vos besoins. N’utilisez pas `--privileged` pour contourner un refus de sandbox : vérifiez les exigences du fournisseur ou utilisez l’installation native. Les mécanismes de sandbox des agents n’ont pas tous été validés dans cette image.

## 3. Utiliser le CLI dans Docker

Le serveur et le CLI lisent la même base du projet :

```sh
docker compose --env-file deploy/install.env exec swarm \
  swarm --root /workspace work list

docker compose --env-file deploy/install.env exec swarm \
  swarm --root /workspace --json work list
```

Si le serveur est arrêté, exécutez une commande ponctuelle :

```sh
docker compose --env-file deploy/install.env run --rm --no-deps swarm work list
```

## 4. Données, arrêt et mise à jour

| Données | Emplacement sur l’hôte | Emplacement dans Docker |
| --- | --- | --- |
| Missions, historique, SQLite, configuration et clés API | `PROJET/.swarm/` | `/workspace/.swarm/` |
| Fichiers du projet | Votre dossier projet | `/workspace/` |
| Outils et authentifications des agents | `~/.local/share/swarm/agents` par défaut | `/home/swarm/` |
| Configuration de l’installation | `deploy/install.env` | Utilisée par Compose |

Les clés API sont stockées dans `.swarm/ai-connections.json`, avec des permissions `0600`, sans chiffrement au repos. Ne versionnez ni les clés ni les données de mission.

**Avant d’arrêter ou de recréer le conteneur, mettez les missions en pause puis arrêtez leurs agents depuis Swarm.** Fermer le navigateur ne coupe pas les agents. En revanche, arrêter le conteneur termine aussi ses processus agents : la phrase du serveur sur la persistance des agents concerne le processus web, pas la destruction de son conteneur.

```sh
# Après arrêt des missions et agents :
docker compose --env-file deploy/install.env stop

# Puis supprimer seulement le conteneur, sans supprimer les dossiers montés :
docker compose --env-file deploy/install.env down
```

Pour une sauvegarde cohérente, arrêtez les agents et le conteneur, puis sauvegardez le projet avec son dossier `.swarm/` et le dossier personnel des agents. Ces sauvegardes contiennent potentiellement des secrets. Une simple copie du seul fichier SQLite pendant son utilisation n’est pas la procédure proposée.

Mise à jour depuis le dépôt, après sauvegarde :

```sh
git pull --ff-only
docker compose --env-file deploy/install.env build
docker compose --env-file deploy/install.env up -d --wait
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

Ne faites pas fonctionner deux serveurs Swarm sur la même base de mission. Un retour à un ancien binaire peut aussi nécessiter de restaurer une sauvegarde compatible avec son schéma.

## 5. Installation native

Prérequis : Linux, Git, Bash, Go 1.24+ et un accès réseau pour les dépendances Go. Node.js n’est pas nécessaire pour compiler le binaire avec les ressources web déjà présentes.

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
./install.sh --mode native --project "$HOME/projets/mon-projet"
"$HOME/.local/bin/swarm" --root "$HOME/projets/mon-projet" providers init
"$HOME/.local/bin/swarm" --root "$HOME/projets/mon-projet" web 127.0.0.1:18787
```

Le dossier projet doit déjà exister. Si `providers.json` existe déjà, ne relancez pas son initialisation : inspectez et adaptez le fichier existant. Les agents utilisent leurs installations et authentifications locales.

Pour choisir un autre emplacement du binaire :

```sh
./install.sh --mode native --bin-dir "$HOME/outils/bin"
```

Le script compile dans un fichier temporaire du dossier cible, puis remplace le binaire lorsque la compilation a réussi. Il ne démarre aucun serveur en mode natif. Arrêtez votre serveur avant de mettre à jour le binaire qu’il utilise.

## Dépannage

| Symptôme | Vérification |
| --- | --- |
| Docker inaccessible | `docker info` et permissions de votre utilisateur |
| Port déjà occupé | Choisir `--port 18788` ou arrêter l’autre service après avoir vérifié son rôle |
| Aucun lien de session | `docker compose --env-file deploy/install.env ps` puis `logs --tail 30 swarm` |
| Permission refusée sur le projet | UID/GID dans `deploy/install.env`, droits des dossiers sur l’hôte |
| Fournisseur absent | Installation dans le conteneur, PATH et `.swarm/providers.json` |
| Agent installé mais appel refusé | Authentification, variables autorisées et capacités de sandbox |
| Ancienne mission avec chemins invalides | Adapter les profils vers `/workspace` sans lancer deux serveurs sur la même base |
| Méthode APEX, KS ou PDCA absente | Installer les ressources de méthode dans le projet piloté, voir [les limites](docs/migration/README.md) |

Le Dockerfile compile les sources présentes localement. Il n’existe pas ici de promesse d’image publique préconstruite ni de compatibilité universelle avec les fournisseurs.

## Recette de l’installateur

Dans une copie du dépôt sans `deploy/install.env`, `make test-install` exécute une recette isolée. Elle compile le binaire natif, construit l’image, ouvre une session web, crée une mission depuis le CLI et vérifie sa conservation après recréation du conteneur. Elle contrôle aussi la propriété des fichiers et le refus de réinstaller un service actif. Aucun modèle IA n’est appelé.

Le test nécessite un moteur Docker local ayant accès aux mêmes chemins que le shell, ainsi que Go et Python 3. Il crée puis retire uniquement son projet, son conteneur et sa configuration temporaires. Les images construites restent disponibles dans Docker. Il ne vérifie pas les authentifications ni les sandboxes de tous les fournisseurs.
