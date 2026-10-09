# Recommencer une installation de Swarm

[English](en/FRESH-INSTALL.md) · [Guide complet](../INSTALL.md)

Cette recette Linux sépare le code de Swarm, le projet piloté et les données des
agents. Chaque collègue utilise ses propres dossiers et connexions. Une
installation neuve ne doit pas réutiliser une base `.swarm` de démonstration.

## Installation native

Prérequis : Git, Bash, Python 3 et Go 1.24 ou supérieur. Depuis un dossier où
`swarm` n’existe pas encore :

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projets/swarm-essai"
./install.sh --mode native --bin-dir "$HOME/.local/bin" \
  --project "$HOME/projets/swarm-essai"
./swarm.sh configure --root "$HOME/projets/swarm-essai" \
  --binary "$HOME/.local/bin/swarm" --address 127.0.0.1:18792
./swarm.sh start
```

Le navigateur reçoit un lien de session privé. `./swarm.sh open` le reconnecte
si nécessaire ; ne partagez ni le lien de session ni les fichiers de `.swarm`.
L’adresse normale est `http://127.0.0.1:18792/`. Un autre serveur présent sur ce
port est refusé ; choisissez alors une autre adresse avec `configure`.

Pour contrôler, relancer puis arrêter cette installation :

```sh
./swarm.sh status
./swarm.sh restart
./swarm.sh logs
./swarm.sh stop
```

`restart` conserve les données du projet et les connexions. Le lanceur se trouve
à la racine du clone, à côté de `install.sh` ; exécutez ces commandes depuis ce
dossier. En natif, les outils des agents doivent être installés et authentifiés
sur le poste selon le [guide d’installation](../INSTALL.md#2-configurer-les-ia-dans-docker).

## Installation Docker

Prérequis supplémentaires : Docker Engine local sur Linux, accessible à votre
utilisateur, et Compose v2. Choisissez Docker ou natif pour un même port.

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projets/swarm-essai-docker"
./install.sh --project "$HOME/projets/swarm-essai-docker" \
  --agent-home "$HOME/.local/share/swarm/agents-essai" --port 18787
docker compose --env-file deploy/install.env ps
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

Ouvrez le lien de session affiché dans les logs ; ils restent privés. Le projet
est monté dans `/workspace`, le dossier des agents dans `/home/swarm`. Le dossier
du code Swarm n’est pas le projet utilisateur. L’image ne contient pas de
fournisseur d’agent préinstallé ou de clé IA.

```sh
docker compose --env-file deploy/install.env restart swarm
docker compose --env-file deploy/install.env down
docker compose --env-file deploy/install.env up -d --wait
```

Ces commandes conservent les deux dossiers montés. `swarm.sh` est le lanceur
**natif** ; les commandes Compose gèrent l’installation Docker. Un
`compose.override.yaml` privé peut ajouter les certificats d’une IA sur site :
suivez la section des overrides dans le [guide d’installation](../INSTALL.md).
Le serveur et ses certificats doivent être accessibles depuis le conteneur.
Ne désactivez pas la vérification TLS pour masquer une autorité manquante.

## Recette à faire dans l’IHM

1. Ouvrir le cockpit, puis **Préparer un projet**.
2. Donner un nom et un besoin, puis **Enregistrer le besoin**.
3. Ouvrir **Préparer avec l’IA**. **Analyse et planification** correspond à APEX.
4. Vérifier les cinq méthodes : analyse et planification, parcours guidé,
   diagnostic, examen et amélioration. Elles sont embarquées dans Swarm ; un
   projet vide n’a pas besoin d’un dossier `.claude` ou d’une installation Claude.
   Les profils de consignes du projet sont distincts : dans un projet vide,
   `.claude`, `AGENTS.md` ou les autres profils peuvent être indiqués
   **indisponibles**. Cela ne rend pas les méthodes embarquées indisponibles.
5. Ajouter votre propre IA et utiliser **Tester la connexion**. En cas d’échec,
   copier les diagnostics en masquant les informations internes avant partage.
6. Envoyer une demande de préparation. Elle propose du texte et un plan ; elle
   ne réalise pas l’application, ne lance pas d’agent et ne valide pas de résultat.
7. Relire et adopter le brief, puis vérifier le plan et l’équipe. L’exécution
   demande ensuite un fournisseur avec outils, des limites et une autorisation.
8. Redémarrer avec la commande du mode choisi. Retrouver le besoin enregistré
   et les connexions ; ne pas saisir à nouveau une clé déjà conservée.

Le [guide utilisateur](../GUIDE-UTILISATEUR.md) explique les étapes suivantes et
les graphes. La [formation pizza FR/EN](training/casa-pizza/README.md) fournit un
atelier détaillé. Une installation ou une méthode disponible ne prouve pas la
réussite d’une mission réelle avec un fournisseur IA.

Les sources livrées sont versionnées dans [`.claude/skills`](../.claude/skills),
les commandes de préparation dans [`.claude/commands`](../.claude/commands) et
le [contrat partagé](../tools/agent-workflows/CONTRACT.md). Le build les intègre
dans le binaire et l’image ; elles restent accessibles si le projet utilisateur
ne contient aucun de ces fichiers.

## Contrôler les méthodes depuis le terminal

```sh
# Natif : utiliser exactement le binaire installé et la racine du serveur.
"$HOME/.local/bin/swarm" --root "$HOME/projets/swarm-essai" --json prepare methods

# Docker : depuis le clone qui contient deploy/install.env.
docker compose --env-file deploy/install.env exec -T swarm \
  swarm --root /workspace --json prepare methods
```

Les cinq entrées doivent contenir `available: true` et une empreinte `sha256`.
Si une méthode locale personnalisée existe, elle est prioritaire. Un lien
symbolique, un fichier invalide ou un accès refusé reste une erreur explicite ;
corrigez ce fichier plutôt que de changer le périmètre du projet. Voir les
[règles des méthodes](AGENT-METHODS.md).

Pour mettre à jour un clone existant, arrêtez proprement les missions et le
serveur, sauvegardez le projet, puis utilisez `git pull --ff-only`. Si Git refuse,
conservez les modifications locales et examinez la divergence. Reconstruisez avec
`install.sh` dans le mode choisi, puis relancez. Un `git pull` seul ne remplace
pas un binaire natif déjà installé ou un conteneur existant.

[Parcours produit KS et vues des graphes](PRODUCT-WORKFLOW.md).
